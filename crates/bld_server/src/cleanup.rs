use std::{io::ErrorKind, sync::Arc, time::Duration};

use actix_web::rt::spawn;
use anyhow::Result;
use bld_config::BldConfig;
use bld_models::{artifacts, login_attempts};
use sea_orm::{DatabaseConnection, TransactionTrait};
use tokio::{
    fs::{remove_dir, remove_file},
    task::JoinHandle,
    time::sleep,
};
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

        let tx = match conn.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                warn!(
                    "unable to begin transaction for artifact {} cleanup due to {e}",
                    artifact.id
                );
                continue;
            }
        };

        if let Err(e) = artifacts::delete_by_id(&tx, &artifact.id).await {
            error!(
                "unable to delete expired artifact entry {} due to {e}",
                artifact.id
            );
            continue; // continue, drop tx and rollback
        }

        match remove_file(&path).await {
            Ok(_) => debug!("removed artifact file at {path:?}"),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                info!("artifact file not found on disk at {path:?}, removing database entry only");
            }
            Err(e) => {
                warn!("unable to remove artifact file at {path:?}: {e}");
                continue; // continue, drop tx and rollback
            }
        }

        if let Err(e) = tx.commit().await {
            warn!(
                "unable to commit transaction during artifact {} cleanup due to {e}",
                artifact.id
            );
        }

        if let Some(parent) = path.parent()
            && let Err(e) = remove_dir(parent).await
            && !matches!(e.kind(), ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty)
        {
            warn!(
                "unable to remove artifact directory for run {} due to {e}",
                artifact.run_id
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cleanup_expired_artifacts;
    use bld_config::BldConfig;
    use bld_models::{
        artifacts::{self, Artifacts, InsertArtifact},
        new_connection_pool,
        pipeline_runs::{self, InsertPipelineRun},
    };
    use sea_orm::{ConnectionTrait, DatabaseConnection};
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
    };
    use tokio::fs::{create_dir_all, write};
    use uuid::Uuid;

    async fn setup() -> (DatabaseConnection, Arc<BldConfig>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("bld-cleanup-{}", Uuid::new_v4()));
        create_dir_all(&dir).await.unwrap();

        let mut config = BldConfig {
            root_dir: dir.display().to_string(),
            ..Default::default()
        };
        config.local.server.db = Some(format!("sqlite://{}/bld.db?mode=rwc", dir.display()));

        let config = Arc::new(config);
        let conn = new_connection_pool(config.clone()).await.unwrap();
        (conn, config, dir)
    }

    async fn add_run(conn: &DatabaseConnection, run_id: &str) {
        let run = InsertPipelineRun {
            id: run_id.to_string(),
            name: "test-pipeline".to_string(),
            app_user: "test-user".to_string(),
        };
        pipeline_runs::insert(conn, run).await.unwrap();
    }

    async fn expire(conn: &DatabaseConnection, id: &str) {
        conn.execute_unprepared(&format!(
            "UPDATE artifacts SET date_expires = '2000-01-01 00:00:00' WHERE id = '{id}'"
        ))
        .await
        .unwrap();
    }

    async fn add_artifact(
        conn: &DatabaseConnection,
        config: &BldConfig,
        run_id: &str,
        expired: bool,
    ) -> Artifacts {
        let insert = InsertArtifact {
            run_id: run_id.to_string(),
            name: format!("artifact-{}", Uuid::new_v4()),
        };
        let model = artifacts::insert(conn, insert, 7).await.unwrap();

        let path = config.artifact_full_path(run_id, &model.id);
        create_dir_all(path.parent().unwrap()).await.unwrap();
        write(&path, b"artifact").await.unwrap();

        if expired {
            expire(conn, &model.id).await;
        }

        model
    }

    async fn row_exists(conn: &DatabaseConnection, id: &str) -> bool {
        artifacts::select_by_id(conn, id).await.is_ok()
    }

    fn cleanup_dir(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn removes_expired_file_row_and_empty_run_directory() {
        let (conn, config, dir) = setup().await;
        let run_id = Uuid::new_v4().to_string();
        add_run(&conn, &run_id).await;
        let artifact = add_artifact(&conn, &config, &run_id, true).await;

        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(!config.artifact_full_path(&run_id, &artifact.id).exists());
        assert!(!row_exists(&conn, &artifact.id).await);
        assert!(!config.artifacts_run_dir(&run_id).exists());

        cleanup_dir(&dir);
    }

    #[tokio::test]
    async fn keeps_run_directory_while_it_has_other_artifacts() {
        let (conn, config, dir) = setup().await;
        let run_id = Uuid::new_v4().to_string();
        add_run(&conn, &run_id).await;
        let expired = add_artifact(&conn, &config, &run_id, true).await;
        let active = add_artifact(&conn, &config, &run_id, false).await;

        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(!config.artifact_full_path(&run_id, &expired.id).exists());
        assert!(!row_exists(&conn, &expired.id).await);
        assert!(config.artifact_full_path(&run_id, &active.id).is_file());
        assert!(row_exists(&conn, &active.id).await);
        assert!(config.artifacts_run_dir(&run_id).is_dir());

        expire(&conn, &active.id).await;
        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(!row_exists(&conn, &active.id).await);
        assert!(!config.artifacts_run_dir(&run_id).exists());

        cleanup_dir(&dir);
    }

    #[tokio::test]
    async fn removes_row_when_file_is_already_missing() {
        let (conn, config, dir) = setup().await;
        let run_id = Uuid::new_v4().to_string();
        add_run(&conn, &run_id).await;
        let artifact = add_artifact(&conn, &config, &run_id, true).await;
        std::fs::remove_file(config.artifact_full_path(&run_id, &artifact.id)).unwrap();

        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(!row_exists(&conn, &artifact.id).await);

        cleanup_dir(&dir);
    }

    #[tokio::test]
    async fn keeps_row_when_file_removal_fails() {
        let (conn, config, dir) = setup().await;
        let run_id = Uuid::new_v4().to_string();
        add_run(&conn, &run_id).await;
        let artifact = add_artifact(&conn, &config, &run_id, true).await;

        let path = config.artifact_full_path(&run_id, &artifact.id);
        std::fs::remove_file(&path).unwrap();
        create_dir_all(&path).await.unwrap();
        write(path.join("blocker"), b"blocker").await.unwrap();

        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(row_exists(&conn, &artifact.id).await);
        assert!(config.artifacts_run_dir(&run_id).is_dir());

        cleanup_dir(&dir);
    }

    #[tokio::test]
    async fn handles_several_runs_in_one_cycle() {
        let (conn, config, dir) = setup().await;
        let run_x = Uuid::new_v4().to_string();
        let run_y = Uuid::new_v4().to_string();
        add_run(&conn, &run_x).await;
        add_run(&conn, &run_y).await;
        add_artifact(&conn, &config, &run_x, true).await;
        add_artifact(&conn, &config, &run_y, true).await;
        let active = add_artifact(&conn, &config, &run_y, false).await;

        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(!config.artifacts_run_dir(&run_x).exists());
        assert!(config.artifacts_run_dir(&run_y).is_dir());
        assert!(config.artifact_full_path(&run_y, &active.id).is_file());
        assert!(row_exists(&conn, &active.id).await);

        cleanup_dir(&dir);
    }

    #[tokio::test]
    async fn does_nothing_when_no_artifact_is_expired() {
        let (conn, config, dir) = setup().await;
        let run_id = Uuid::new_v4().to_string();
        add_run(&conn, &run_id).await;
        let artifact = add_artifact(&conn, &config, &run_id, false).await;

        cleanup_expired_artifacts(&conn, &config).await.unwrap();

        assert!(config.artifact_full_path(&run_id, &artifact.id).is_file());
        assert!(row_exists(&conn, &artifact.id).await);
        assert!(config.artifacts_run_dir(&run_id).is_dir());

        cleanup_dir(&dir);
    }
}
