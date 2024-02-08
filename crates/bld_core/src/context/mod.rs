pub mod local;
pub mod run;
pub mod server;

use crate::platform::PlatformSender;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_models::pipeline_run_containers::PipelineRunContainers;
use run::RemoteRun;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::oneshot;

use self::local::{LocalContextBackend, LocalContextMessage};
use self::server::{ServerContextBackend, ServerContextMessage};

pub enum Context {
    Server(Sender<ServerContextMessage>),
    Local(Sender<LocalContextMessage>),
}

impl Context {
    pub fn server(config: Arc<BldConfig>, pool: Arc<DatabaseConnection>, run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        ServerContextBackend::new(config, pool, run_id, rx).receive();
        Self::Server(tx)
    }

    pub fn local(config: Arc<BldConfig>) -> Self {
        let (tx, rx) = channel(4096);
        LocalContextBackend::new(config, rx).receive();
        Self::Local(tx)
    }

    pub async fn add_remote_run(&self, server: String, run_id: String) -> Result<()> {
        let remote_run = RemoteRun::new(server, run_id);
        match self {
            Self::Server(tx) => tx
                .send(ServerContextMessage::AddRemoteRun(remote_run))
                .await
                .map_err(|e| anyhow!(e)),
            Self::Local(tx) => tx
                .send(LocalContextMessage::AddRemoteRun(remote_run))
                .await
                .map_err(|e| anyhow!(e)),
        }
    }

    pub async fn remove_remote_run(&self, run_id: &str) -> Result<()> {
        match self {
            Self::Server(tx) => tx
                .send(ServerContextMessage::RemoveRemoteRun(run_id.to_owned()))
                .await
                .map_err(|e| anyhow!(e)),
            Self::Local(tx) => tx
                .send(LocalContextMessage::RemoveRemoteRun(run_id.to_owned()))
                .await
                .map_err(|e| anyhow!(e)),
        }
    }

    pub async fn add_platform(&self, platform: Arc<PlatformSender>) -> Result<()> {
        match self {
            Self::Server(tx) => tx
                .send(ServerContextMessage::AddPlatform(platform))
                .await
                .map_err(|e| anyhow!(e)),
            Self::Local(tx) => tx
                .send(LocalContextMessage::AddPlatform(platform))
                .await
                .map_err(|e| anyhow!(e)),
        }
    }

    pub async fn remove_platform(&self, platform_id: &str) -> Result<()> {
        match self {
            Self::Server(tx) => tx
                .send(ServerContextMessage::RemovePlatform(
                    platform_id.to_string(),
                ))
                .await
                .map_err(|e| anyhow!(e)),
            Self::Local(tx) => tx
                .send(LocalContextMessage::RemovePlatform(platform_id.to_string()))
                .await
                .map_err(|e| anyhow!(e)),
        }
    }

    pub async fn set_pipeline_as_running(&self, run_id: String) -> Result<()> {
        let Self::Server(tx) = self else {
            return Ok(());
        };

        tx.send(ServerContextMessage::SetPipelineAsRunning(run_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_pipeline_as_finished(&self, run_id: String) -> Result<()> {
        let Self::Server(tx) = self else {
            return Ok(());
        };

        tx.send(ServerContextMessage::SetPipelineAsFinished(run_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_pipeline_as_faulted(&self, run_id: String) -> Result<()> {
        let Self::Server(tx) = self else {
            return Ok(());
        };

        tx.send(ServerContextMessage::SetPipelineAsFaulted(run_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn add_container(
        &self,
        container_id: String,
    ) -> Result<Option<PipelineRunContainers>> {
        let Self::Server(tx) = self else {
            return Ok(None);
        };

        let (resp_tx, resp_rx) = oneshot::channel();

        tx.send(ServerContextMessage::AddContainer {
            container_id,
            resp_tx,
        })
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

        resp_rx.await.map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_container_as_removed(&self, id: String) -> Result<()> {
        let Self::Server(tx) = self else {
            return Ok(());
        };

        tx.send(ServerContextMessage::SetContainerAsRemoved(id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_container_as_faulted(&self, id: String) -> Result<()> {
        let Self::Server(tx) = self else {
            return Ok(());
        };

        tx.send(ServerContextMessage::SetContainerAsFaulted(id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn keep_alive(&self, id: String) -> Result<()> {
        let Self::Server(tx) = self else {
            return Ok(());
        };

        tx.send(ServerContextMessage::KeepAliveContainer(id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn cleanup(&self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        match self {
            Self::Server(tx) => tx
                .send(ServerContextMessage::DoCleanup(resp_tx))
                .await
                .map_err(|e| anyhow!(e))?,
            Self::Local(tx) => tx
                .send(LocalContextMessage::DoCleanup(resp_tx))
                .await
                .map_err(|e| anyhow!(e))?,
        }

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
