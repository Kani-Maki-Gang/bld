pub mod builder;
mod container;
mod context;
mod docker;
mod image;
mod machine;
mod ssh;

use std::sync::Arc;

pub use container::*;
pub use context::*;
pub use docker::*;
use futures::channel::oneshot;
pub use image::*;
pub use machine::*;
pub use ssh::*;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use uuid::Uuid;

use actix::spawn;
use anyhow::{Result, anyhow};
use tracing::{debug, error};

use crate::logger::Logger;

pub enum PlatformArtifactsAction {
    Push,
    Get,
}

pub enum PlatformMessage {
    Artifacts {
        action: PlatformArtifactsAction,
        from: String,
        to: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Shell {
        logger: Arc<Logger>,
        working_dir: Option<String>,
        command: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Dispose {
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

pub enum PlatformType {
    Machine(Box<Machine>),
    Container(Box<Container>),
    Ssh(Sender<PlatformMessage>),
    Mock
}

struct PlatformBackend {
    ssh: Box<Ssh>,
    receiver: Receiver<PlatformMessage>,
}

impl PlatformBackend {
    pub fn new(ssh: Box<Ssh>, receiver: Receiver<PlatformMessage>) -> Self {
        Self { ssh, receiver }
    }

    pub async fn receive(mut self) -> Result<()> {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                PlatformMessage::Artifacts {
                    action,
                    from,
                    to,
                    resp_tx,
                } => {
                    let res = match action {
                        PlatformArtifactsAction::Push => self.push(from, to).await,
                        PlatformArtifactsAction::Get => self.get(from, to).await,
                    };
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }

                PlatformMessage::Shell {
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

                PlatformMessage::Dispose { resp_tx } => {
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
        debug!("executing push operation");
        self.ssh.copy_into(&from, &to).await
    }

    pub async fn get(&mut self, from: String, to: String) -> Result<()> {
        debug!("executing get operation");
        self.ssh.copy_from(&from, &to).await
    }

    pub async fn shell(
        &self,
        logger: Arc<Logger>,
        working_dir: Option<String>,
        command: String,
    ) -> Result<()> {
        self.ssh.sh(logger, &working_dir, &command).await
    }

    pub async fn dispose(&mut self) -> Result<()> {
        self.ssh.dispose().await
    }
}

pub struct Platform {
    id: String,
    inner: PlatformType,
}

impl Platform {
    pub fn machine(machine: Box<Machine>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            inner: PlatformType::Machine(machine),
        }
    }

    pub fn container(container: Box<Container>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            inner: PlatformType::Container(container),
        }
    }

    pub fn ssh(ssh: Box<Ssh>) -> Self {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = channel(4096);

        spawn(async move {
            let receiver = PlatformBackend::new(ssh, rx);
            if let Err(e) = receiver.receive().await {
                error!("{e}");
            }
        });

        Self {
            id,
            inner: PlatformType::Ssh(tx),
        }
    }

    pub fn mock() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            inner: PlatformType::Mock,
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn is(&self, pid: &str) -> bool {
        self.id == pid
    }

    pub async fn push(&self, from: &str, to: &str) -> Result<()> {
        match &self.inner {
            PlatformType::Machine(machine) => machine.copy_into(from, to).await,
            PlatformType::Container(container) => container.copy_into(from, to).await,
            PlatformType::Ssh(ssh) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh.send(PlatformMessage::Artifacts {
                    action: PlatformArtifactsAction::Push,
                    from: from.to_string(),
                    to: to.to_string(),
                    resp_tx,
                })
                .await?;

                resp_rx.await?
            }
            PlatformType::Mock => Ok(())
        }
    }

    pub async fn get(&self, from: &str, to: &str) -> Result<()> {
        match &self.inner {
            PlatformType::Machine(machine) => machine.copy_from(from, to).await,
            PlatformType::Container(container) => container.copy_from(from, to).await,
            PlatformType::Ssh(ssh) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh.send(PlatformMessage::Artifacts {
                    action: PlatformArtifactsAction::Get,
                    from: from.to_string(),
                    to: to.to_string(),
                    resp_tx,
                })
                .await?;

                resp_rx.await?
            }
            PlatformType::Mock => Ok(())
        }
    }

    pub async fn shell(
        &self,
        logger: Arc<Logger>,
        working_dir: &Option<String>,
        command: &str,
    ) -> Result<()> {
        match &self.inner {
            PlatformType::Machine(machine) => machine.sh(logger, working_dir, command).await,
            PlatformType::Container(container) => container.sh(logger, working_dir, command).await,
            PlatformType::Ssh(ssh) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh.send(PlatformMessage::Shell {
                    logger,
                    working_dir: working_dir.clone(),
                    command: command.to_string(),
                    resp_tx,
                })
                .await?;

                resp_rx.await?
            }
            PlatformType::Mock => Ok(())
        }
    }

    pub async fn keep_alive(&self) -> Result<()> {
        match &self.inner {
            PlatformType::Container(container) => container.keep_alive().await,
            _ => Ok(()),
        }
    }

    pub async fn dispose(&self, in_child_runner: bool) -> Result<()> {
        match &self.inner {
            // checking if the runner is a child in order to not cleanup the temp dir for the whole run
            PlatformType::Machine(machine) if !in_child_runner => machine.dispose().await,
            PlatformType::Machine(_) => Ok(()),
            PlatformType::Container(container) => container.dispose().await,
            PlatformType::Ssh(ssh) => {
                let (resp_tx, resp_rx) = oneshot::channel();
                ssh.send(PlatformMessage::Dispose { resp_tx }).await?;
                resp_rx.await?
            }
            PlatformType::Mock => Ok(())
        }
    }
}
