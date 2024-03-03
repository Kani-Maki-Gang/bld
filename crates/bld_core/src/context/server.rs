use super::run::RemoteRun;
use crate::platform::Platform;
use actix::spawn;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_http::Request;
use bld_models::{
    pipeline_run_containers::{
        self, InsertPipelineRunContainer, PipelineRunContainers, PRC_STATE_FAULTED,
        PRC_STATE_KEEP_ALIVE, PRC_STATE_REMOVED,
    },
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_RUNNING},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};
use tracing::{debug, error};
use uuid::Uuid;

pub enum ServerContextMessage {
    AddRemoteRun(RemoteRun),
    RemoveRemoteRun(String),
    AddPlatform(Arc<Platform>),
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

pub struct ServerContextBackend {
    config: Arc<BldConfig>,
    run_id: String,
    remote_runs: Vec<RemoteRun>,
    conn: Arc<DatabaseConnection>,
    platforms: Vec<Arc<Platform>>,
    rx: Receiver<ServerContextMessage>,
}

impl ServerContextBackend {
    pub fn new(
        config: Arc<BldConfig>,
        conn: Arc<DatabaseConnection>,
        run_id: &str,
        rx: Receiver<ServerContextMessage>,
    ) -> Self {
        Self {
            config,
            run_id: run_id.to_owned(),
            remote_runs: vec![],
            conn,
            platforms: vec![],
            rx,
        }
    }

    pub async fn update_pipeline_state(&self, run_id: &str, state: &str) -> Result<()> {
        pipeline_runs::update_state(self.conn.as_ref(), run_id, state)
            .await
            .map(|_| ())
    }

    pub fn receive(self) {
        spawn(async move {
            if let Err(e) = self.receive_inner().await {
                error!("{e}");
            }
        });
    }

    pub async fn receive_inner(mut self) -> Result<()> {
        while let Some(message) = self.rx.recv().await {
            match message {
                ServerContextMessage::AddRemoteRun(remote_run) => {
                    debug!("added new remote run");
                    self.remote_runs.push(remote_run);
                }

                ServerContextMessage::RemoveRemoteRun(run_id) => {
                    self.remote_runs.retain(|rr| rr.run_id != run_id);
                }

                ServerContextMessage::AddPlatform(platform) => {
                    self.platforms.push(platform);
                }

                ServerContextMessage::RemovePlatform(platform_id) => {
                    self.platforms.retain(|p| !p.is(&platform_id));
                }

                ServerContextMessage::SetPipelineAsRunning(run_id) => {
                    self.update_pipeline_state(&run_id, PR_STATE_RUNNING)
                        .await?;
                }

                ServerContextMessage::SetPipelineAsFinished(run_id) => {
                    self.update_pipeline_state(&run_id, PR_STATE_FINISHED)
                        .await?;
                }

                ServerContextMessage::SetPipelineAsFaulted(run_id) => {
                    self.update_pipeline_state(&run_id, PR_STATE_FAULTED)
                        .await?;
                }

                ServerContextMessage::AddContainer {
                    container_id,
                    resp_tx,
                } => self.add_container(&container_id, resp_tx).await?,

                ServerContextMessage::SetContainerAsRemoved(entity_id) => {
                    pipeline_run_containers::update_state(
                        self.conn.as_ref(),
                        &entity_id,
                        PRC_STATE_REMOVED,
                    )
                    .await?;
                }

                ServerContextMessage::SetContainerAsFaulted(entity_id) => {
                    pipeline_run_containers::update_state(
                        self.conn.as_ref(),
                        &entity_id,
                        PRC_STATE_FAULTED,
                    )
                    .await?;
                }

                ServerContextMessage::KeepAliveContainer(entity_id) => {
                    pipeline_run_containers::update_state(
                        self.conn.as_ref(),
                        &entity_id,
                        PRC_STATE_KEEP_ALIVE,
                    )
                    .await?;
                }

                ServerContextMessage::DoCleanup(resp_tx) => self.do_cleanup(resp_tx).await?,
            }
        }
        Ok(())
    }

    async fn add_container(
        &mut self,
        container_id: &str,
        resp_tx: oneshot::Sender<Option<PipelineRunContainers>>,
    ) -> Result<()> {
        let entity = pipeline_run_containers::insert(
            self.conn.as_ref(),
            InsertPipelineRunContainer {
                id: Uuid::new_v4().to_string(),
                run_id: self.run_id.to_owned(),
                container_id: container_id.to_owned(),
                state: "active".to_owned(),
            },
        )
        .await
        .map_err(|e| error!("{e}"))
        .ok();

        resp_tx
            .send(entity)
            .map_err(|_| anyhow!("oneshot response sender dropped"))?;

        Ok(())
    }

    async fn do_cleanup(&mut self, resp_tx: oneshot::Sender<()>) -> Result<()> {
        self.update_pipeline_state(&self.run_id, PR_STATE_FAULTED)
            .await?;

        for run in self.remote_runs.iter() {
            let _ = self
                .cleanup_remote_run(run)
                .await
                .map_err(|e| error!("{e}"));
        }

        for platform in self.platforms.iter() {
            let _ = platform.dispose(false).await.map_err(|e| error!("{e}"));
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    async fn cleanup_remote_run(&self, run: &RemoteRun) -> Result<()> {
        let server = self.config.server(&run.server)?;
        let auth_path = self.config.auth_full_path(&server.name);
        let url = format!("{}/stop", server.base_url_http());

        let _: String = Request::post(&url)
            .auth(&auth_path)
            .await
            .send_json(&run.run_id)
            .await?;

        Ok(())
    }
}
