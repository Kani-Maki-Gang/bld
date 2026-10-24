pub mod builder;
mod container;
mod context;
mod docker;
mod image;
mod machine;
mod ssh;

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub use container::*;
pub use context::*;
pub use docker::*;
use futures::channel::oneshot;
pub use image::*;
pub use machine::*;
pub use ssh::*;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use uuid::Uuid;

use actix_web::rt::spawn;
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
        shell: Option<String>,
        working_dir: Option<String>,
        command: String,
        resp_tx: oneshot::Sender<Result<HashMap<String, String>>>,
    },
    Dispose {
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

pub enum PlatformType {
    Machine(Box<Machine>),
    Container(Box<Container>),
    Ssh(Sender<PlatformMessage>),
    Mock,
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
                    shell,
                    working_dir,
                    command,
                    resp_tx,
                } => {
                    let res = self.shell(logger, shell, working_dir, command).await;
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
        shell: Option<String>,
        working_dir: Option<String>,
        command: String,
    ) -> Result<HashMap<String, String>> {
        self.ssh
            .sh(logger, shell.as_deref(), &working_dir, &command)
            .await
    }

    pub async fn dispose(&mut self) -> Result<()> {
        self.ssh.dispose().await
    }
}

pub struct Platform {
    id: String,
    shell: Option<String>,
    inner: PlatformType,
    disposed: AtomicBool,
}

impl Platform {
    pub fn machine(machine: Box<Machine>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            shell: None,
            disposed: AtomicBool::new(false),
            inner: PlatformType::Machine(machine),
        }
    }

    pub fn container(container: Box<Container>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            shell: None,
            disposed: AtomicBool::new(false),
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
            shell: None,
            disposed: AtomicBool::new(false),
            inner: PlatformType::Ssh(tx),
        }
    }

    pub fn mock() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            shell: None,
            disposed: AtomicBool::new(false),
            inner: PlatformType::Mock,
        }
    }

    pub fn with_shell(mut self, shell: Option<String>) -> Self {
        self.shell = shell;
        self
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
            PlatformType::Mock => Ok(()),
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
            PlatformType::Mock => Ok(()),
        }
    }

    pub async fn shell(
        &self,
        logger: Arc<Logger>,
        working_dir: &Option<String>,
        command: &str,
    ) -> Result<HashMap<String, String>> {
        let shell = self.shell.as_deref();
        match &self.inner {
            PlatformType::Machine(machine) => machine.sh(logger, shell, working_dir, command).await,
            PlatformType::Container(container) => {
                container.sh(logger, shell, working_dir, command).await
            }
            PlatformType::Ssh(ssh) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                ssh.send(PlatformMessage::Shell {
                    logger,
                    shell: self.shell.clone(),
                    working_dir: working_dir.clone(),
                    command: command.to_string(),
                    resp_tx,
                })
                .await?;

                resp_rx.await?
            }
            PlatformType::Mock => Ok(HashMap::new()),
        }
    }

    pub async fn keep_alive(&self) -> Result<()> {
        match &self.inner {
            PlatformType::Container(container) => container.keep_alive().await,
            _ => Ok(()),
        }
    }

    pub async fn dispose(&self, in_child_runner: bool) -> Result<()> {
        // checking if the runner is a child in order to not cleanup the temp dir for the whole run
        let is_child_machine = in_child_runner && matches!(self.inner, PlatformType::Machine(_));
        if is_child_machine || self.disposed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        match &self.inner {
            PlatformType::Machine(machine) => machine.dispose().await,
            PlatformType::Container(container) => container.dispose().await,
            PlatformType::Ssh(ssh) => {
                let (resp_tx, resp_rx) = oneshot::channel();
                ssh.send(PlatformMessage::Dispose { resp_tx }).await?;
                resp_rx.await?
            }
            PlatformType::Mock => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::Path};

    use bld_config::BldConfig;
    use bld_utils::sync::IntoArc;
    use uuid::Uuid;

    use super::{Machine, Platform};

    #[tokio::test]
    async fn disposing_a_platform_twice_succeeds() {
        let config = BldConfig {
            root_dir: std::env::temp_dir().display().to_string(),
            ..Default::default()
        }
        .into_arc();
        let id = format!("platform-dispose-twice-test-{}", Uuid::new_v4());
        let tmp_dir = config.tmp_full_path(&id);
        let machine = Machine::new(&id, config, &HashMap::new(), HashMap::new().into_arc())
            .await
            .unwrap();
        let platform = Platform::machine(Box::new(machine));

        platform.dispose(false).await.unwrap();
        assert!(!Path::new(&tmp_dir).exists());
        platform.dispose(false).await.unwrap();
    }
}
