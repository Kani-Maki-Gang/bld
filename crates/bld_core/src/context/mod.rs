use crate::database::pipeline_run_containers::{
    self, InsertPipelineRunContainer, PipelineRunContainers, PRC_STATE_FAULTED,
    PRC_STATE_KEEP_ALIVE, PRC_STATE_REMOVED,
};
use crate::database::pipeline_runs::{self, PR_STATE_FINISHED, PR_STATE_RUNNING};
use crate::platform::TargetPlatform;
use crate::request::Request;
use actix_web::rt::spawn;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tracing::{debug, error};
use uuid::Uuid;

pub enum ContextMessage {
    AddRemoteRun(RemoteRun),
    RemoveRemoteRun(String),
    AddPlatform(Arc<TargetPlatform>),
    RemovePlatform(String),
    SetPipelineAsRunning(String),
    SetPipelineAsFinished(String),
    SetPipelineAsFaulted(String),
    AddContainer {
        container_id: String,
        resp_tx: oneshot::Sender<Option<PipelineRunContainers>>,
    },
    SetContainerAsRemoved(String),
    SetContainerAsFaulted(String),
    KeepAliveContainer(String),
    DoCleanup(oneshot::Sender<()>),
}

#[derive(Debug, Clone)]
pub struct RemoteRun {
    pub server: String,
    pub run_id: String,
}

impl RemoteRun {
    pub fn new(server: String, run_id: String) -> Self {
        Self { server, run_id }
    }
}

#[derive(Clone)]
pub enum Context {
    Server {
        config: Arc<BldConfig>,
        run_id: String,
        remote_runs: Vec<RemoteRun>,
        conn: Arc<DatabaseConnection>,
        platforms: Vec<Arc<TargetPlatform>>,
    },
    Local {
        config: Arc<BldConfig>,
        remote_runs: Vec<RemoteRun>,
        platforms: Vec<Arc<TargetPlatform>>,
    },
}

impl Context {
    pub fn server(config: Arc<BldConfig>, conn: Arc<DatabaseConnection>, run_id: &str) -> Self {
        Self::Server {
            config,
            run_id: run_id.to_owned(),
            remote_runs: vec![],
            conn,
            platforms: vec![],
        }
    }

    pub fn local(config: Arc<BldConfig>) -> Self {
        Self::Local {
            config,
            remote_runs: vec![],
            platforms: vec![],
        }
    }

    pub async fn update_pipeline_state(&self, run_id: &str, state: &str) -> Result<()> {
        if let Self::Server { conn, .. } = self {
            pipeline_runs::update_state(conn.as_ref(), run_id, state)
                .await
                .map(|_| ())?;
        }
        Ok(())
    }

    pub async fn receive(mut self, mut rx: Receiver<ContextMessage>) -> Result<()> {
        while let Some(message) = rx.recv().await {
            match message {
                ContextMessage::AddRemoteRun(remote_run) => self.add_remote_run(remote_run),

                ContextMessage::RemoveRemoteRun(run_id) => self.remove_remote_run(run_id),

                ContextMessage::AddPlatform(platform) => self.add_platform(platform),

                ContextMessage::RemovePlatform(platform_id) => self.remove_platform(&platform_id),

                ContextMessage::SetPipelineAsRunning(run_id) => {
                    self.set_pipeline_as_running(&run_id).await?;
                }

                ContextMessage::SetPipelineAsFinished(run_id) => {
                    self.set_pipeline_as_finished(&run_id).await?;
                }

                ContextMessage::SetPipelineAsFaulted(run_id) => {
                    self.set_pipeline_as_faulted(&run_id).await?;
                }

                ContextMessage::AddContainer {
                    container_id,
                    resp_tx,
                } => self.add_container(&container_id, resp_tx).await?,

                ContextMessage::SetContainerAsRemoved(entity_id) => {
                    self.set_container_as_removed(&entity_id).await?;
                }

                ContextMessage::SetContainerAsFaulted(entity_id) => {
                    self.set_container_as_faulted(&entity_id).await?;
                }

                ContextMessage::KeepAliveContainer(entity_id) => {
                    self.keep_alive_container(&entity_id).await?;
                }

                ContextMessage::DoCleanup(resp_tx) => self.do_cleanup(resp_tx).await?,
            }
        }
        Ok(())
    }

    fn add_remote_run(&mut self, remote_run: RemoteRun) {
        match self {
            Self::Server { remote_runs, .. } | Self::Local { remote_runs, .. } => {
                debug!("added new remote run");
                remote_runs.push(remote_run);
            }
        }
    }

    fn remove_remote_run(&mut self, run_id: String) {
        match self {
            Self::Server { remote_runs, .. } | Self::Local { remote_runs, .. } => {
                remote_runs.retain(|rr| rr.run_id != run_id);
            }
        }
    }

    fn add_platform(&mut self, platform: Arc<TargetPlatform>) {
        match self {
            Self::Server { platforms, .. } | Self::Local { platforms, .. } => {
                platforms.push(platform);
            }
        }
    }

    fn remove_platform(&mut self, platform_id: &str) {
        match self {
            Self::Server { platforms, .. } | Self::Local { platforms, .. } => {
                platforms.retain(|p| !p.is(platform_id));
            }
        }
    }

    async fn set_pipeline_as_running(&self, run_id: &str) -> Result<()> {
        self.update_pipeline_state(run_id, PR_STATE_RUNNING).await
    }

    async fn set_pipeline_as_finished(&self, run_id: &str) -> Result<()> {
        self.update_pipeline_state(run_id, PR_STATE_FINISHED).await
    }

    async fn set_pipeline_as_faulted(&self, run_id: &str) -> Result<()> {
        self.update_pipeline_state(run_id, PRC_STATE_FAULTED).await
    }

    async fn add_container(
        &mut self,
        container_id: &str,
        resp_tx: oneshot::Sender<Option<PipelineRunContainers>>,
    ) -> Result<()> {
        let mut entity = None;

        if let Self::Server { run_id, conn, .. } = self {
            entity = pipeline_run_containers::insert(
                conn.as_ref(),
                InsertPipelineRunContainer {
                    id: Uuid::new_v4().to_string(),
                    run_id: run_id.to_owned(),
                    container_id: container_id.to_owned(),
                    state: "active".to_owned(),
                },
            )
            .await
            .map_err(|e| error!("{e}"))
            .ok();
        }

        resp_tx
            .send(entity)
            .map_err(|_| anyhow!("oneshot response sender dropped"))?;

        Ok(())
    }

    async fn set_container_as_removed(&mut self, entity_id: &str) -> Result<()> {
        if let Self::Server { conn, .. } = self {
            pipeline_run_containers::update_state(conn.as_ref(), entity_id, PRC_STATE_REMOVED)
                .await?;
        }
        Ok(())
    }

    async fn set_container_as_faulted(&mut self, entity_id: &str) -> Result<()> {
        if let Self::Server { conn, .. } = self {
            pipeline_run_containers::update_state(conn.as_ref(), entity_id, PRC_STATE_FAULTED)
                .await?;
        };

        Ok(())
    }

    async fn keep_alive_container(&mut self, entity_id: &str) -> Result<()> {
        if let Self::Server { conn, .. } = self {
            pipeline_run_containers::update_state(conn.as_ref(), entity_id, PRC_STATE_KEEP_ALIVE)
                .await?;
        }
        Ok(())
    }

    async fn do_cleanup(&mut self, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match &self {
            Self::Server {
                config,
                run_id,
                remote_runs,
                platforms,
                ..
            } => {
                self.set_pipeline_as_faulted(run_id).await?;

                for run in remote_runs.iter() {
                    let _ = Self::cleanup_remote_run(config.clone(), run)
                        .await
                        .map_err(|e| error!("{e}"));
                }

                for platform in platforms.iter() {
                    let _ = platform.dispose(false).await.map_err(|e| error!("{e}"));
                }
            }

            Self::Local {
                config,
                remote_runs,
                platforms,
                ..
            } => {
                for run in remote_runs.iter() {
                    let _ = Self::cleanup_remote_run(config.clone(), run)
                        .await
                        .map_err(|e| error!("{e}"));
                }
                for platform in platforms.iter() {
                    let _ = platform.dispose(false).await.map_err(|e| error!("{e}"));
                }
            }
        }
        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    async fn cleanup_remote_run(config: Arc<BldConfig>, run: &RemoteRun) -> Result<()> {
        let server = config.server(&run.server)?;
        let auth_path = config.auth_full_path(&server.name);
        let url = format!("{}/stop", server.base_url_http());

        let _: String = Request::post(&url)
            .auth(&auth_path)
            .await
            .send_json(&run.run_id)
            .await?;

        Ok(())
    }
}

pub struct ContextSender {
    tx: Sender<ContextMessage>,
}

impl ContextSender {
    pub fn server(config: Arc<BldConfig>, pool: Arc<DatabaseConnection>, run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        let context = Context::server(config, pool, run_id);

        spawn(async move {
            if let Err(e) = context.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx }
    }

    pub fn local(config: Arc<BldConfig>) -> Self {
        let (tx, rx) = channel(4096);
        let context = Context::local(config);

        spawn(async move {
            if let Err(e) = context.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx }
    }

    pub async fn add_remote_run(&self, server: String, run_id: String) -> Result<()> {
        let remote_run = RemoteRun::new(server, run_id);

        self.tx
            .send(ContextMessage::AddRemoteRun(remote_run))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn remove_remote_run(&self, run_id: &str) -> Result<()> {
        self.tx
            .send(ContextMessage::RemoveRemoteRun(run_id.to_owned()))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn add_platform(&self, platform: Arc<TargetPlatform>) -> Result<()> {
        self.tx
            .send(ContextMessage::AddPlatform(platform))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn remove_platform(&self, platform_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::RemovePlatform(platform_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_pipeline_as_running(&self, run_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetPipelineAsRunning(run_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_pipeline_as_finished(&self, run_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetPipelineAsFinished(run_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_pipeline_as_faulted(&self, run_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetPipelineAsFaulted(run_id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn add_container(
        &self,
        container_id: String,
    ) -> Result<Option<PipelineRunContainers>> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(ContextMessage::AddContainer {
                container_id,
                resp_tx,
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        resp_rx.await.map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_container_as_removed(&self, id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetContainerAsRemoved(id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn set_container_as_faulted(&self, id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetContainerAsFaulted(id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn keep_alive(&self, id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::KeepAliveContainer(id))
            .await
            .map_err(|e| anyhow!("{e}"))
    }

    pub async fn cleanup(&self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(ContextMessage::DoCleanup(resp_tx))
            .await
            .map_err(|e| anyhow!("{e}"))?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
