mod container;
mod image;
mod machine;
mod ssh;

use std::sync::Arc;

pub use container::*;
use futures::channel::oneshot;
pub use image::*;
pub use machine::*;
pub use ssh::*;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use uuid::Uuid;

use actix::spawn;
use anyhow::{anyhow, Result};
use tracing::{debug, error};

use crate::logger::LoggerSender;

pub enum TargetPlatformMessage {
    Push {
        from: String,
        to: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Get {
        from: String,
        to: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Shell {
        logger: Arc<LoggerSender>,
        working_dir: Option<String>,
        command: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Dispose {
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

struct TargetPlatformReceiver {
    ssh: Box<Ssh>,
    receiver: Receiver<TargetPlatformMessage>,
}

impl TargetPlatformReceiver {
    pub fn new(ssh: Box<Ssh>, receiver: Receiver<TargetPlatformMessage>) -> Self {
        Self { ssh, receiver }
    }

    pub async fn receive(mut self) -> Result<()> {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                TargetPlatformMessage::Push { from, to, resp_tx } => {
                    debug!("executing push operation");
                    let res = self.push(from, to).await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }

                TargetPlatformMessage::Get { from, to, resp_tx } => {
                    debug!("executing get operation");
                    let res = self.get(from, to).await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }

                TargetPlatformMessage::Shell {
                    logger,
                    working_dir,
                    command,
                    resp_tx,
                } => {
                    let res = self.shell(logger, working_dir, command).await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }

                TargetPlatformMessage::Dispose { resp_tx } => {
                    let res = self.dispose().await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }
            }
        }
        Ok(())
    }

    pub async fn push(&mut self, from: String, to: String) -> Result<()> {
        self.ssh.copy_into(&from, &to).await
    }

    pub async fn get(&mut self, from: String, to: String) -> Result<()> {
        self.ssh.copy_from(&from, &to).await
    }

    pub async fn shell(
        &mut self,
        logger: Arc<LoggerSender>,
        working_dir: Option<String>,
        command: String,
    ) -> Result<()> {
        self.ssh.as_mut().sh(logger, &working_dir, &command).await
    }

    pub async fn dispose(&mut self) -> Result<()> {
        self.ssh.dispose().await
    }
}

pub enum TargetPlatform {
    Machine {
        id: String,
        machine: Box<Machine>,
    },
    Container {
        id: String,
        container: Box<Container>,
    },
    Ssh {
        id: String,
        ssh_tx: Sender<TargetPlatformMessage>,
    },
}

impl TargetPlatform {
    pub fn machine(machine: Box<Machine>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self::Machine { id, machine }
    }

    pub fn container(container: Box<Container>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self::Container { id, container }
    }

    pub fn ssh(ssh: Box<Ssh>) -> Self {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = channel(4096);

        spawn(async move {
            let receiver = TargetPlatformReceiver::new(ssh, rx);
            if let Err(e) = receiver.receive().await {
                error!("{e}");
            }
        });

        Self::Ssh { id, ssh_tx: tx }
    }

    pub fn id(&self) -> String {
        match self {
            Self::Machine { id, .. } | Self::Container { id, .. } | Self::Ssh { id, .. } => {
                id.to_owned()
            }
        }
    }

    pub fn is(&self, pid: &str) -> bool {
        match self {
            Self::Machine { id, .. } | Self::Container { id, .. } | Self::Ssh { id, .. } => {
                pid == id
            }
        }
    }

    pub async fn push(&self, from: &str, to: &str) -> Result<()> {
        match self {
            Self::Machine { machine, .. } => machine.copy_into(from, to).await,
            Self::Container { container, .. } => container.copy_into(from, to).await,
            Self::Ssh { ssh_tx, .. } => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh_tx
                    .send(TargetPlatformMessage::Push {
                        from: from.to_string(),
                        to: to.to_string(),
                        resp_tx,
                    })
                    .await?;

                resp_rx.await?
            }
        }
    }

    pub async fn get(&self, from: &str, to: &str) -> Result<()> {
        match self {
            Self::Machine { machine, .. } => machine.copy_from(from, to).await,
            Self::Container { container, .. } => container.copy_from(from, to).await,
            Self::Ssh { ssh_tx, .. } => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh_tx
                    .send(TargetPlatformMessage::Get {
                        from: from.to_string(),
                        to: to.to_string(),
                        resp_tx,
                    })
                    .await?;

                resp_rx.await?
            }
        }
    }

    pub async fn shell(
        &self,
        logger: Arc<LoggerSender>,
        working_dir: &Option<String>,
        command: &str,
    ) -> Result<()> {
        match self {
            Self::Machine { machine, .. } => machine.sh(logger, working_dir, command).await,
            Self::Container { container, .. } => container.sh(logger, working_dir, command).await,
            Self::Ssh { ssh_tx, .. } => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh_tx
                    .send(TargetPlatformMessage::Shell {
                        logger,
                        working_dir: working_dir.clone(),
                        command: command.to_string(),
                        resp_tx,
                    })
                    .await?;

                resp_rx.await?
            }
        }
    }

    pub async fn keep_alive(&self) -> Result<()> {
        match self {
            Self::Container { container, .. } => container.keep_alive().await,
            _ => Ok(()),
        }
    }

    pub async fn dispose(&self, in_child_runner: bool) -> Result<()> {
        match self {
            // checking if the runner is a child in order to not cleanup the temp dir for the whole run
            Self::Machine { machine, .. } if !in_child_runner => machine.dispose().await,
            Self::Machine { .. } => Ok(()),
            Self::Container { container, .. } => container.dispose().await,
            Self::Ssh { ssh_tx, .. } => {
                let (resp_tx, resp_rx) = oneshot::channel();
                ssh_tx
                    .send(TargetPlatformMessage::Dispose { resp_tx })
                    .await?;
                resp_rx.await?
            }
        }
    }
}
