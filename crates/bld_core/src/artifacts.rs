use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_web::rt::spawn;
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use flate2::{Compression, write::GzEncoder};
use tar::Builder;
use tokio::fs::{create_dir_all, remove_dir_all, write};
use tokio::sync::{
    mpsc::{Receiver, Sender, channel},
    oneshot,
};
use tracing::error;
use uuid::Uuid;

use crate::platform::Platform;

enum ArtifactsMessage {
    Download {
        platform: Arc<Platform>,
        name: String,
        remote_path: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Upload {
        platform: Arc<Platform>,
        name: String,
        local_path: String,
        remote_path: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

struct ArtifactsBackend {
    config: Arc<BldConfig>,
    run_id: String,
    map: HashMap<String, PathBuf>,
    rx: Receiver<ArtifactsMessage>,
}

impl ArtifactsBackend {
    fn new(config: Arc<BldConfig>, run_id: String, rx: Receiver<ArtifactsMessage>) -> Self {
        Self {
            config,
            run_id,
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
                    remote_path,
                    resp_tx,
                } => {
                    let res = self.download(&platform, &name, &remote_path).await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }
                ArtifactsMessage::Upload {
                    platform,
                    name,
                    local_path,
                    remote_path,
                    resp_tx,
                } => {
                    let res = self
                        .upload(&platform, &name, &local_path, &remote_path)
                        .await;
                    resp_tx
                        .send(res)
                        .map_err(|_| anyhow!("oneshot channel closed"))?;
                }
            }
        }
        Ok(())
    }

    fn local_path(&mut self, name: &str) -> &PathBuf {
        let run_id = &self.run_id;
        let config = &self.config;
        self.map
            .entry(name.to_string())
            .or_insert_with(|| config.artifact_full_path(run_id, name))
    }

    async fn download(&mut self, platform: &Platform, name: &str, remote_path: &str) -> Result<()> {
        let staging_dir = self.config.tmp_full_path(&Uuid::new_v4().to_string());
        create_dir_all(&staging_dir).await?;

        let result = self
            .download_inner(platform, name, remote_path, &staging_dir)
            .await;

        if let Err(e) = remove_dir_all(&staging_dir).await {
            error!("unable to clean up staging directory for artifact {name}: {e}");
        }

        result
    }

    async fn download_inner(
        &mut self,
        platform: &Platform,
        name: &str,
        remote_path: &str,
        staging_dir: &Path,
    ) -> Result<()> {
        platform
            .get(remote_path, &staging_dir.display().to_string())
            .await?;

        let archive_path = self.local_path(name);
        if let Some(parent) = archive_path.parent() {
            create_dir_all(parent).await?;
        }

        let compressed = compress_to_tar_gz(staging_dir, name)?;
        write(archive_path, compressed).await?;

        Ok(())
    }

    async fn upload(
        &mut self,
        platform: &Platform,
        name: &str,
        local_path: &str,
        remote_path: &str,
    ) -> Result<()> {
        let archive_path = self.local_path(name);
        if let Some(parent) = archive_path.parent() {
            create_dir_all(parent).await?;
        }

        let compressed = compress_to_tar_gz(Path::new(local_path), name)?;
        write(archive_path, compressed).await?;

        platform
            .push(&archive_path.display().to_string(), remote_path)
            .await
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
    pub fn new(config: Arc<BldConfig>, run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        ArtifactsBackend::new(config, run_id.to_string(), rx).receive();
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

    pub async fn download(
        &self,
        platform: Arc<Platform>,
        name: &str,
        remote_path: &str,
    ) -> Result<()> {
        let Some(tx) = &self.tx else { return Ok(()) };
        let (resp_tx, resp_rx) = oneshot::channel();

        tx.send(ArtifactsMessage::Download {
            platform,
            name: name.to_string(),
            remote_path: remote_path.to_string(),
            resp_tx,
        })
        .await?;

        resp_rx.await?
    }

    pub async fn upload(
        &self,
        platform: Arc<Platform>,
        name: &str,
        local_path: &str,
        remote_path: &str,
    ) -> Result<()> {
        let Some(tx) = &self.tx else { return Ok(()) };
        let (resp_tx, resp_rx) = oneshot::channel();

        tx.send(ArtifactsMessage::Upload {
            platform,
            name: name.to_string(),
            local_path: local_path.to_string(),
            remote_path: remote_path.to_string(),
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
