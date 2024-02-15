use crate::platform::Platform;
use actix::spawn;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_http::Request;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};
use tracing::{debug, error};

use super::run::RemoteRun;

pub enum LocalContextMessage {
    AddRemoteRun(RemoteRun),
    RemoveRemoteRun(String),
    AddPlatform(Arc<Platform>),
    RemovePlatform(String),
    DoCleanup(oneshot::Sender<()>),
}

pub struct LocalContextBackend {
    config: Arc<BldConfig>,
    remote_runs: Vec<RemoteRun>,
    platforms: Vec<Arc<Platform>>,
    rx: Receiver<LocalContextMessage>,
}

impl LocalContextBackend {
    pub fn new(config: Arc<BldConfig>, rx: Receiver<LocalContextMessage>) -> Self {
        Self {
            config,
            remote_runs: vec![],
            platforms: vec![],
            rx,
        }
    }

    pub fn receive(self) {
        spawn(async move {
            if let Err(e) = self.receive_inner().await {
                error!("{e}");
            }
        });
    }

    async fn receive_inner(mut self) -> Result<()> {
        while let Some(message) = self.rx.recv().await {
            match message {
                LocalContextMessage::AddRemoteRun(remote_run) => {
                    debug!("added new remote run");
                    self.remote_runs.push(remote_run);
                }

                LocalContextMessage::RemoveRemoteRun(run_id) => {
                    self.remote_runs.retain(|rr| rr.run_id != run_id);
                }

                LocalContextMessage::AddPlatform(platform) => {
                    self.platforms.push(platform);
                }

                LocalContextMessage::RemovePlatform(platform_id) => {
                    self.platforms.retain(|p| !p.is(&platform_id));
                }

                LocalContextMessage::DoCleanup(resp_tx) => self.do_cleanup(resp_tx).await?,
            }
        }
        Ok(())
    }

    async fn do_cleanup(&mut self, resp_tx: oneshot::Sender<()>) -> Result<()> {
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
