use actix::io::SinkWrite;
use actix::{Actor, StreamHandler};
use actix_web::rt::spawn;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_sock::clients::EnqueueClient;
use bld_sock::messages::ServerMessages;
use bld_utils::request::WebSocket;
use futures::stream::StreamExt;
use std::env::current_exe;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error};


struct SupervisorMessageReceiver {
    config: Arc<BldConfig>,
    rx: Receiver<ServerMessages>,
    child: Option<Child>,
    last_message: Option<ServerMessages>,
}

impl SupervisorMessageReceiver {
    pub fn new(config: Arc<BldConfig>, rx: Receiver<ServerMessages>) -> Self {
        Self {
            config,
            rx,
            child: None,
            last_message: None,
        }
    }

    pub async fn receive(mut self) -> Result<()> {
        'retry_loop: loop {
            if let Err(e) = self.spawn_inner() {
                error!("{e}");
                break 'retry_loop;
            }

            // small wait for the supervisor
            sleep(Duration::from_millis(300)).await;

            let supervisor = &self.config.local.supervisor;
            let url = format!(
                "{}://{}:{}/ws-server/",
                supervisor.ws_protocol(),
                supervisor.host,
                supervisor.port
            );

            debug!("establishing web socket connection on {}", url);

            let (_, framed) = WebSocket::new(&url)?
                .request()
                .connect()
                .await
                .map_err(|e| {
                    error!("{e}");
                    anyhow!(e.to_string())
                })?;

            let (sink, stream) = framed.split();
            let address = EnqueueClient::create(|ctx| {
                EnqueueClient::add_stream(stream, ctx);
                EnqueueClient::new(SinkWrite::new(sink, ctx))
            });

            address.send(ServerMessages::Ack).await.map_err(|e| {
                error!("failed to send Ack to supervisor, {e}");
                e
            })?;

            if let Some(msg) = self.last_message {
                address.send(msg).await?;
                self.last_message = None;
            }

            while let Some(msg) = self.rx.recv().await {
                if !address.connected() {
                    self.kill_inner().await;
                    self.last_message = Some(msg);
                    continue 'retry_loop;
                }

                address.send(msg).await?;
            }

            self.kill_inner().await;
        }

        Ok(())
    }

    fn spawn_inner(&mut self) -> Result<()> {
        current_exe()
            .and_then(|exe| Command::new(exe).arg("supervisor").spawn())
            .map(|child| {
                self.child = Some(child);
                debug!("spawned new supervisor process");
            })
            .map_err(|e| anyhow!(e))
    }

    async fn kill_inner(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
        }
    }
}

pub struct SupervisorMessageSender {
    pub tx: Sender<ServerMessages>,
    _rx_task: JoinHandle<()>,
}

impl SupervisorMessageSender {
    pub fn new(config: Arc<BldConfig>) -> Self {
        let (tx, rx) = channel(4096);
        let receiver = SupervisorMessageReceiver::new(config, rx);

        let rx_task = spawn(async move {
            if let Err(e) = receiver.receive().await {
                error!("{e}");
            }
        });

        Self {
            tx,
            _rx_task: rx_task,
        }
    }

    pub async fn enqueue(
        &self,
        pipeline: String,
        run_id: String,
        variables: Option<Vec<String>>,
        environment: Option<Vec<String>>,
    ) -> Result<()> {
        let message = ServerMessages::Enqueue {
            pipeline,
            run_id,
            variables,
            environment,
        };

        self.tx.send(message).await.map_err(|e| anyhow!(e))
    }
}
