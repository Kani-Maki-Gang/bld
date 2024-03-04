use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_codec::Framed;
use actix_http::ws::Codec;
use actix_web::rt::spawn;
use anyhow::{anyhow, bail, Result};
use awc::BoxedSocket;
use bld_config::BldConfig;
use bld_http::WebSocket;
use bld_models::dtos::ServerMessages;
use bld_sock::EnqueueClient;
use futures::stream::StreamExt;
use std::{env::current_exe, sync::Arc, time::Duration};
use tokio::{
    process::{Child, Command},
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, error};

const INITIAL_DELAY: u64 = 500;
const RETRY_DELAY: u64 = 2000;

async fn try_ws_connection(url: &str) -> Result<Framed<BoxedSocket, Codec>> {
    // small wait for the supervisor
    sleep(Duration::from_millis(INITIAL_DELAY)).await;

    for _ in 0..10 {
        debug!("establishing web socket connection on {}", url);

        let Ok((_, framed)) = WebSocket::new(&url)?
            .request()
            .connect()
            .await
            .map_err(|e| {
                error!("connection to supervisor web socket failed due to {e}, retrying in {RETRY_DELAY}ms");
                anyhow!(e.to_string())
            }) else {
                sleep(Duration::from_millis(RETRY_DELAY)).await;
                continue;
            };

        return Ok(framed);
    }

    bail!("unable to establish connection to supervisor websocket");
}

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
        let supervisor = &self.config.local.supervisor;
        let url = format!("{}/v1/ws-server/", supervisor.base_url_ws());

        'retry_loop: loop {
            if let Err(e) = self.spawn_inner() {
                error!("{e}");
                break 'retry_loop;
            }
            let framed = try_ws_connection(&url).await?;
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

    pub async fn stop(&self, run_id: &str) -> Result<()> {
        let message = ServerMessages::Stop {
            run_id: run_id.to_owned(),
        };

        self.tx.send(message).await.map_err(|e| anyhow!(e))
    }
}
