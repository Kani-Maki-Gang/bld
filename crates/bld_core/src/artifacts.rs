use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_web::rt::spawn;
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use bld_models::artifacts::{self, InsertArtifact};
use flate2::{Compression, write::GzEncoder};
use sea_orm::DatabaseConnection;
use tar::Builder;
use tokio::fs::{create_dir_all, remove_dir_all, write};
use tokio::sync::{
    mpsc::{Receiver, Sender, channel},
    oneshot,
};
use tracing::error;
use uuid::Uuid;

use crate::platform::Platform;

pub enum ArtifactsStore {
    Local,
    Server(Arc<DatabaseConnection>),
}

enum ArtifactsMessage {
    Download {
        platform: Arc<Platform>,
        name: String,
        to: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Upload {
        platform: Arc<Platform>,
        name: String,
        path: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

struct ArtifactsBackend {
    config: Arc<BldConfig>,
    run_id: String,
    store: ArtifactsStore,
    map: HashMap<String, PathBuf>,
    rx: Receiver<ArtifactsMessage>,
}

impl ArtifactsBackend {
    fn new(
        config: Arc<BldConfig>,
        run_id: String,
        store: ArtifactsStore,
        rx: Receiver<ArtifactsMessage>,
    ) -> Self {
        Self {
            config,
            run_id,
            store,
            map: HashMap::new(),
            rx,
        }
    }

    fn receive(self) {
        spawn(async move {
            if let Err(e) = self.receive_inner().await {
                error!("{e}");
            }
        });
    }

    async fn receive_inner(mut self) -> Result<()> {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                ArtifactsMessage::Download {
                    platform,
                    name,
                    to,
                    resp_tx,
                } => {
                    let res = self.download(&platform, name, to).await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }
                ArtifactsMessage::Upload {
                    platform,
                    name,
                    path,
                    resp_tx,
                } => {
                    let res = self.upload(&platform, name, path).await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }
            }
        }
        Ok(())
    }

    async fn download(&mut self, platform: &Platform, name: String, to: String) -> Result<()> {
        let archive_path = self
            .map
            .get(&name)
            .ok_or_else(|| anyhow!("artifact '{name}' not found"))?;
        platform
            .push(&archive_path.display().to_string(), &to)
            .await
    }

    async fn upload(&mut self, platform: &Platform, name: String, path: String) -> Result<()> {
        let staging_dir = self.config.tmp_full_path(&Uuid::new_v4().to_string());
        create_dir_all(&staging_dir).await?;

        let result = self
            .upload_inner(platform, &name, &path, &staging_dir)
            .await;

        if let Err(e) = remove_dir_all(&staging_dir).await {
            error!("unable to clean up staging directory for artifact {name}: {e}");
        }

        result
    }

    async fn upload_inner(
        &mut self,
        platform: &Platform,
        name: &str,
        path: &str,
        staging_dir: &Path,
    ) -> Result<()> {
        platform
            .get(path, &staging_dir.display().to_string())
            .await?;

        let artifact_path = match self.map.get(name) {
            Some(value) => value.clone(),
            None => {
                let path = self.resolve_artifact_path(name).await?;
                self.map.insert(name.to_string(), path.clone());
                path
            }
        };

        if let Some(parent) = artifact_path.parent() {
            create_dir_all(parent).await?;
        }

        let compressed = compress_to_tar_gz(staging_dir, name)?;
        write(&artifact_path, compressed).await?;

        Ok(())
    }

    async fn resolve_artifact_path(&self, name: &str) -> Result<PathBuf> {
        let file_name = match &self.store {
            ArtifactsStore::Local => name,
            ArtifactsStore::Server(conn) => {
                let insert = InsertArtifact {
                    run_id: self.run_id.clone(),
                    name: name.to_string(),
                };
                let model = artifacts::insert(conn.as_ref(), insert).await?;
                &model.id.to_string()
            }
        };

        let artifact_path = self.config.artifact_full_path(&self.run_id, file_name);

        Ok(artifact_path)
    }
}

fn compress_to_tar_gz(source: &Path, entry_name: &str) -> Result<Vec<u8>> {
    let mut tar = Builder::new(Vec::new());

    if source.is_file() {
        tar.append_path_with_name(source, entry_name)?;
    } else {
        tar.append_dir_all(entry_name, source)?;
    }

    let uncompressed = tar.into_inner()?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&uncompressed)?;
    Ok(gz.finish()?)
}

pub struct Artifacts {
    tx: Option<Sender<ArtifactsMessage>>,
}

impl Artifacts {
    pub fn new(config: Arc<BldConfig>, run_id: &str, store: ArtifactsStore) -> Self {
        let (tx, rx) = channel(4096);
        ArtifactsBackend::new(config, run_id.to_string(), store, rx).receive();
        Self { tx: Some(tx) }
    }

    pub fn mock() -> Self {
        Self { tx: None }
    }

    pub async fn cleanup_run(config: &BldConfig, run_id: &str) -> Result<()> {
        let dir = config.artifacts_run_dir(run_id);
        if dir.is_dir() {
            remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    pub async fn download(&self, platform: Arc<Platform>, name: &str, to: &str) -> Result<()> {
        let Some(tx) = &self.tx else { return Ok(()) };
        let (resp_tx, resp_rx) = oneshot::channel();

        tx.send(ArtifactsMessage::Download {
            platform,
            name: name.to_string(),
            to: to.to_string(),
            resp_tx,
        })
        .await?;

        resp_rx.await?
    }

    pub async fn upload(&self, platform: Arc<Platform>, name: &str, path: &str) -> Result<()> {
        let Some(tx) = &self.tx else { return Ok(()) };
        let (resp_tx, resp_rx) = oneshot::channel();

        tx.send(ArtifactsMessage::Upload {
            platform,
            name: name.to_string(),
            path: path.to_string(),
            resp_tx,
        })
        .await?;

        resp_rx.await?
    }
}

#[cfg(test)]
mod tests {
    use super::compress_to_tar_gz;
    use bld_config::BldConfig;
    use flate2::read::GzDecoder;
    use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
    use tar::Archive;
    use uuid::Uuid;

    #[test]
    fn compress_to_tar_gz_round_trip() {
        let config = BldConfig::default();
        let base = config.tmp_full_path(&format!("artifacts-test-{}", Uuid::new_v4()));
        let source = base.join("payload");
        let extracted = base.join("extracted");
        create_dir_all(&source).unwrap();
        write(source.join("hello.txt"), b"hello world").unwrap();

        let compressed = compress_to_tar_gz(&source, "my-artifact").unwrap();

        let tar = GzDecoder::new(&compressed[..]);
        let mut archive = Archive::new(tar);
        archive.unpack(&extracted).unwrap();

        let content = read_to_string(extracted.join("my-artifact").join("hello.txt")).unwrap();
        assert_eq!(content, "hello world");

        let _ = remove_dir_all(&base);
    }
}
