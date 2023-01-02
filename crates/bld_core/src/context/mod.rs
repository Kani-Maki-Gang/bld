use crate::database::pipeline_run_containers::{
    self, InsertPipelineRunContainer, PipelineRunContainers, PRC_STATE_FAULTED,
    PRC_STATE_KEEP_ALIVE, PRC_STATE_REMOVED,
};
use crate::platform::TargetPlatform;
use actix_web::rt::spawn;
use anyhow::{anyhow, Result};
use bld_utils::request::Request;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tracing::error;
use uuid::Uuid;

pub enum ContextMessage {
    AddRemoteRun {
        base_url: String,
        auth_token: String,
        run_id: String,
    },
    RemoveRemoteRun(String),
    AddPlatform(Arc<TargetPlatform>),
    RemovePlatform(Arc<TargetPlatform>),
    AddContainer(String),
    SetContainerAsRemoved(String),
    SetContainerAsFaulted(String),
    KeepAliveContainer(String),
    DoCleanup(oneshot::Sender<()>),
}

#[derive(Debug, Clone)]
pub struct RemoteRun {
    pub base_url: String,
    pub auth_token: String,
    pub run_id: String,
}

impl RemoteRun {
    pub fn new(base_url: String, auth_token: String, run_id: String) -> Self {
        Self {
            base_url,
            auth_token,
            run_id,
        }
    }
}

#[derive(Clone)]
pub enum Context {
    Server {
        run_id: String,
        remote_runs: Vec<RemoteRun>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        instances: Vec<PipelineRunContainers>,
        platforms: Vec<Arc<TargetPlatform>>,
    },
    Local {
        run_id: String,
        remote_runs: Vec<RemoteRun>,
        platforms: Vec<Arc<TargetPlatform>>,
    },
}

impl Context {
    pub fn server(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, run_id: &str) -> Self {
        Self::Server {
            run_id: run_id.to_owned(),
            remote_runs: vec![],
            pool,
            instances: vec![],
            platforms: vec![],
        }
    }

    pub fn local(run_id: &str) -> Self {
        Self::Local {
            run_id: run_id.to_owned(),
            remote_runs: vec![],
            platforms: vec![],
        }
    }

    pub async fn receive(mut self, mut rx: Receiver<ContextMessage>) -> Result<()> {
        while let Some(message) = rx.recv().await {
            match message {
                ContextMessage::AddRemoteRun {
                    base_url,
                    auth_token,
                    run_id,
                } => self.add_remote_run(base_url, auth_token, run_id),
                ContextMessage::RemoveRemoteRun(run_id) => self.remove_remote_run(run_id),
                ContextMessage::AddPlatform(platform) => self.add_platform(platform),
                ContextMessage::RemovePlatform(platform) => self.remove_platform(platform),
                ContextMessage::AddContainer(id) => self.add_container(&id)?,
                ContextMessage::SetContainerAsRemoved(id) => self.set_container_as_removed(&id)?,
                ContextMessage::SetContainerAsFaulted(id) => self.set_container_as_faulted(&id)?,
                ContextMessage::KeepAliveContainer(id) => self.keep_alive_container(&id)?,
                ContextMessage::DoCleanup(resp_tx) => self.do_cleanup(resp_tx).await?,
            }
        }
        Ok(())
    }

    fn add_remote_run(&mut self, base_url: String, auth_token: String, run_id: String) {
        match self {
            Self::Server { remote_runs, .. } | Self::Local { remote_runs, .. } => {
                remote_runs.push(RemoteRun::new(base_url, auth_token, run_id));
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

    fn remove_platform(&mut self, platform: Arc<TargetPlatform>) {
        match self {
            Self::Server { platforms, .. } | Self::Local { platforms, .. } => {
                platforms.retain(|p| !platform.is(&p.id()));
            }
        }
    }

    fn add_container(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Server {
                run_id,
                pool,
                instances,
                ..
            } => {
                let mut conn = pool.get()?;
                let instance = pipeline_run_containers::insert(
                    &mut conn,
                    InsertPipelineRunContainer {
                        id: &Uuid::new_v4().to_string(),
                        run_id: &run_id,
                        container_id,
                        state: "active",
                    },
                )?;
                instances.push(instance);
            }
            _ => {}
        }
        Ok(())
    }

    fn set_container_as_removed(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Server {
                pool, instances, ..
            } => {
                if let Some(idx) = instances
                    .iter()
                    .position(|i| i.container_id == container_id)
                {
                    let mut conn = pool.get()?;
                    instances[idx] = pipeline_run_containers::update_state(
                        &mut conn,
                        &instances[idx].id,
                        PRC_STATE_REMOVED,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn set_container_as_faulted(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Server {
                pool, instances, ..
            } => {
                if let Some(idx) = instances
                    .iter()
                    .position(|i| i.container_id == container_id)
                {
                    let mut conn = pool.get()?;
                    instances[idx] = pipeline_run_containers::update_state(
                        &mut conn,
                        &instances[idx].id,
                        PRC_STATE_FAULTED,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn keep_alive_container(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Server {
                pool, instances, ..
            } => {
                if let Some(idx) = instances
                    .iter()
                    .position(|i| i.container_id == container_id)
                {
                    let mut conn = pool.get()?;
                    instances[idx] = pipeline_run_containers::update_state(
                        &mut conn,
                        &instances[idx].id,
                        PRC_STATE_KEEP_ALIVE,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn do_cleanup(&mut self, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Server { remote_runs, platforms, .. } => {
                for run in remote_runs.iter() {
                    let _ = Self::cleanup_remote_run(&run).await;
                }
                for platform in platforms.iter() {
                    let _ = platform.dispose(false).await;
                }
            }
            Self::Local { remote_runs, platforms, .. } => {
                for run in remote_runs.iter() {
                    let _ = Self::cleanup_remote_run(&run).await;
                }
                for platform in platforms.iter() {
                    let _ = platform.dispose(false).await;
                }
            }
        }
        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    async fn cleanup_remote_run(run: &RemoteRun) -> Result<()> {
        let url = format!("{}/stop", run.base_url);
        let bearer = format!("Bearer {}", run.auth_token);

        let _: String = Request::post(&url)
            .header("Authorization", &bearer)
            .send_json(&run.run_id)
            .await?;

        Ok(())
    }
}

pub struct ContextSender {
    tx: Sender<ContextMessage>,
}

impl ContextSender {
    pub fn server(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        let context = Context::server(pool, run_id);

        spawn(async move {
            if let Err(e) = context.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx }
    }

    pub fn local(run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        let context = Context::local(run_id);

        spawn(async move {
            if let Err(e) = context.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx }
    }

    pub async fn add_remote_run(
        &self,
        base_url: &str,
        auth_token: &str,
        run_id: &str,
    ) -> Result<()> {
        self.tx
            .send(ContextMessage::AddRemoteRun {
                base_url: base_url.to_owned(),
                auth_token: auth_token.to_owned(),
                run_id: run_id.to_owned(),
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn remove_remote_run(&self, run_id: &str) -> Result<()> {
        self.tx
            .send(ContextMessage::RemoveRemoteRun(run_id.to_owned()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn add_platform(&self, platform: Arc<TargetPlatform>) -> Result<()> {
        self.tx
            .send(ContextMessage::AddPlatform(platform))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn remove_platform(&self, platform: Arc<TargetPlatform>) -> Result<()> {
        self.tx
            .send(ContextMessage::RemovePlatform(platform))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn add_container(&self, container_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::AddContainer(container_id))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn set_container_as_removed(&self, container_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetContainerAsRemoved(container_id))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn set_container_as_faulted(&self, container_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::SetContainerAsFaulted(container_id))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn keep_alive(&self, container_id: String) -> Result<()> {
        self.tx
            .send(ContextMessage::KeepAliveContainer(container_id))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn cleanup(&self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(ContextMessage::DoCleanup(resp_tx))
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
