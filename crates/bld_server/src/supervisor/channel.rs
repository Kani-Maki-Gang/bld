use actix_web::rt::spawn;
use anyhow::{Result, anyhow, bail};
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_models::dtos::ServerMessages;
use bld_sock::{EnqueueClient, EnqueueClientState};
use std::{env::current_exe, sync::Arc, time::Duration};
use tokio::{
    process::{Child, Command},
    sync::mpsc::{Receiver, Sender, channel},
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, error};

const INITIAL_DELAY: u64 = 500;
const RETRY_DELAY: u64 = 2000;
const RESPAWN_DELAY: u64 = 5000;

async fn try_ws_connection(url: &str) -> Result<EnqueueClient> {
    // small wait for the supervisor
    sleep(Duration::from_millis(INITIAL_DELAY)).await;

    for _ in 0..10 {
        debug!("establishing web socket connection on {}", url);

        match EnqueueClient::connect(url, Logger::shell()).await {
            Ok(client) => return Ok(client),
            Err(e) => {
                error!(
                    "connection to supervisor web socket failed due to {e}, retrying in {RETRY_DELAY}ms"
                );
                sleep(Duration::from_millis(RETRY_DELAY)).await;
            }
        }
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
                error!("failed to spawn supervisor process: {e}, retrying in {RESPAWN_DELAY}ms");
                sleep(Duration::from_millis(RESPAWN_DELAY)).await;
                continue 'retry_loop;
            }

            let mut client = match try_ws_connection(&url).await {
                Ok(client) => client,
                Err(e) => {
                    error!("{e}, respawning supervisor in {RESPAWN_DELAY}ms");
                    self.kill_inner().await;
                    sleep(Duration::from_millis(RESPAWN_DELAY)).await;
                    continue 'retry_loop;
                }
            };

            if let Some(msg) = self.last_message.take()
                && client.send(&msg).await.is_err()
            {
                self.kill_inner().await;
                self.last_message = Some(msg);
                sleep(Duration::from_millis(RESPAWN_DELAY)).await;
                continue 'retry_loop;
            }

            loop {
                tokio::select! {
                    msg = self.rx.recv() => {
                        let Some(msg) = msg else {
                            break 'retry_loop;
                        };
                        if client.send(&msg).await.is_err() {
                            self.kill_inner().await;
                            self.last_message = Some(msg);
                            continue 'retry_loop;
                        }
                    }
                    state = client.next() => {
                        if let EnqueueClientState::Completed = state {
                            self.kill_inner().await;
                            continue 'retry_loop;
                        }
                    }
                }
            }
        }

        self.kill_inner().await;

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
            inputs: variables,
            env: environment,
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
