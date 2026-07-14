use std::{sync::Arc, time::Duration};

use actix_web::rt::spawn;
use anyhow::Result;
use bld_config::BldConfig;
use bld_models::{artifacts, login_attempts};
use sea_orm::DatabaseConnection;
use tokio::{task::JoinHandle, time::sleep};
use tracing::{debug, error, info, warn};

pub struct CleanupWorker {
    _task: JoinHandle<()>,
}

impl CleanupWorker {
    pub fn new(conn: Arc<DatabaseConnection>, config: Arc<BldConfig>) -> Self {
        let interval = Duration::from_secs(config.local.server.cleanup_interval.max(1) as u64);

        let task = spawn(async move {
            loop {
                if let Err(e) = login_attempts::delete_expired(&conn).await {
                    error!("login attempts cleanup run failed due to: {e}");
                }
                if let Err(e) = cleanup_expired_artifacts(&conn, &config).await {
                    error!("artifacts cleanup run failed due to: {e}");
                }
                sleep(interval).await;
            }
        });

        Self { _task: task }
    }
}

async fn cleanup_expired_artifacts(conn: &DatabaseConnection, config: &BldConfig) -> Result<()> {
    let expired = artifacts::select_expired(conn).await?;
    if expired.is_empty() {
        debug!("no expired artifacts found");
        return Ok(());
    }

    info!("found {} expired artifact(s) to clean up", expired.len());

    for artifact in expired {
        let path = config.artifact_full_path(&artifact.run_id, &artifact.id);
        match tokio::fs::remove_file(&path).await {
            Ok(_) => debug!("removed artifact file at {path:?}"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                info!("artifact file not found on disk at {path:?}, removing database entry only");
            }
            Err(e) => warn!("unable to remove artifact file at {path:?}: {e}"),
        }

        if let Err(e) = artifacts::delete_by_id(conn, &artifact.id).await {
            error!(
                "unable to delete expired artifact entry {}: {e}",
                artifact.id
            );
        }
    }

    Ok(())
}
