use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_web::rt::spawn;
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use bld_models::artifacts::{self, InsertArtifact};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use sea_orm::DatabaseConnection;
use tar::{Archive, Builder};
use tokio::fs::{create_dir_all, remove_dir_all, remove_file};
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
        let staging_dir = self.config.tmp_full_path(&Uuid::new_v4().to_string());
        create_dir_all(&staging_dir).await?;

        let result = self
            .download_inner(platform, &name, &to, &staging_dir)
            .await;

        if let Err(e) = remove_dir_all(&staging_dir).await {
            error!("unable to clean up staging directory for artifact {name}: {e}");
        }

        result
    }

    async fn download_inner(
        &self,
        platform: &Platform,
        name: &str,
        to: &str,
        staging_dir: &Path,
    ) -> Result<()> {
        let archive_path = self
            .map
            .get(name)
            .ok_or_else(|| anyhow!("artifact '{name}' not found"))?;

        decompress_tar_gz(archive_path, staging_dir)?;

        let extracted_path = staging_dir.join(name);

        platform
            .push(&extracted_path.display().to_string(), to)
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

        let result = compress_to_tar_gz(staging_dir, name, &artifact_path);

        if result.is_err()
            && artifact_path.is_file()
            && let Err(e) = remove_file(&artifact_path).await
        {
            error!("unable to remove incomplete archive for artifact {name}: {e}");
        }

        result
    }

    async fn resolve_artifact_path(&self, name: &str) -> Result<PathBuf> {
        let file_name = match &self.store {
            ArtifactsStore::Local => name.to_string(),
            ArtifactsStore::Server(conn) => {
                let insert = InsertArtifact {
                    run_id: self.run_id.clone(),
                    name: name.to_string(),
                };
                let model = artifacts::insert(
                    conn.as_ref(),
                    insert,
                    self.config.local.server.artifacts_retention_days,
                )
                .await?;
                model.id
            }
        };

        let artifact_path = self.config.artifact_full_path(&self.run_id, &file_name);

        Ok(artifact_path)
    }
}

pub fn validate_artifact_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("artifact name cannot be empty"));
    }

    let path = Path::new(name);
    if path.is_absolute() {
        return Err(anyhow!(
            "artifact name '{name}' must not be an absolute path"
        ));
    }

    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => {}
        _ => {
            return Err(anyhow!(
                "artifact name '{name}' must be a single path segment without '.', '..' or path separators"
            ));
        }
    }

    if name.chars().any(|c| c.is_control()) {
        return Err(anyhow!(
            "an artifact name must not contain a control character"
        ));
    }

    Ok(())
}

/// Compresses the source path into the destination archive. The archive is written
/// while it is created, so neither the tar nor the gzip content is kept in memory.
fn compress_to_tar_gz(source: &Path, entry_name: &str, dest: &Path) -> Result<()> {
    let gz = GzEncoder::new(BufWriter::new(File::create(dest)?), Compression::default());
    let mut tar = Builder::new(gz);

    if source.is_file() {
        tar.append_path_with_name(source, entry_name)?;
    } else {
        tar.append_dir_all(entry_name, source)?;
    }

    tar.into_inner()?.finish()?.flush()?;
    Ok(())
}

fn decompress_tar_gz(archive_path: &Path, dest: &Path) -> Result<()> {
    let gz = GzDecoder::new(BufReader::new(File::open(archive_path)?));
    let mut archive = Archive::new(gz);
    archive.unpack(dest)?;
    Ok(())
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
        validate_artifact_name(name)?;

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
        validate_artifact_name(name)?;

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
    use super::{compress_to_tar_gz, decompress_tar_gz, validate_artifact_name};
    use bld_config::BldConfig;
    use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
    use uuid::Uuid;

    #[test]
    fn validate_artifact_name_accepts_single_segment() {
        assert!(validate_artifact_name("my-artifact").is_ok());
        assert!(validate_artifact_name("my_artifact.tar").is_ok());
    }

    #[test]
    fn validate_artifact_name_rejects_empty() {
        assert!(validate_artifact_name("").is_err());
    }

    #[test]
    fn validate_artifact_name_rejects_absolute_paths() {
        assert!(validate_artifact_name("/etc/passwd").is_err());
    }

    #[test]
    fn validate_artifact_name_rejects_parent_dir_traversal() {
        assert!(validate_artifact_name("..").is_err());
        assert!(validate_artifact_name("../../etc/passwd").is_err());
        assert!(validate_artifact_name("foo/../../bar").is_err());
    }

    #[test]
    fn validate_artifact_name_rejects_nested_segments() {
        assert!(validate_artifact_name("foo/bar").is_err());
    }

    #[test]
    fn validate_artifact_name_rejects_current_dir() {
        assert!(validate_artifact_name(".").is_err());
    }

    #[test]
    fn validate_artifact_name_rejects_control_characters() {
        assert!(validate_artifact_name("foo\nbar").is_err());
        assert!(validate_artifact_name("foo\rbar").is_err());
        assert!(validate_artifact_name("foo\tbar").is_err());
    }

    #[test]
    fn compress_to_tar_gz_round_trip() {
        let config = BldConfig::default();
        let base = config.tmp_full_path(&format!("artifacts-test-{}", Uuid::new_v4()));
        let source = base.join("payload");
        let archive = base.join("payload.tar.gz");
        let extracted = base.join("extracted");
        create_dir_all(&source).unwrap();
        write(source.join("hello.txt"), b"hello world").unwrap();

        compress_to_tar_gz(&source, "my-artifact", &archive).unwrap();
        decompress_tar_gz(&archive, &extracted).unwrap();

        let content = read_to_string(extracted.join("my-artifact").join("hello.txt")).unwrap();
        assert_eq!(content, "hello world");

        let _ = remove_dir_all(&base);
    }

    #[test]
    fn compress_to_tar_gz_round_trip_single_file() {
        let config = BldConfig::default();
        let base = config.tmp_full_path(&format!("artifacts-test-{}", Uuid::new_v4()));
        let archive = base.join("payload.tar.gz");
        let extracted = base.join("extracted");
        create_dir_all(&base).unwrap();
        let source = base.join("payload.txt");
        write(&source, b"hello file").unwrap();

        compress_to_tar_gz(&source, "my-artifact", &archive).unwrap();
        decompress_tar_gz(&archive, &extracted).unwrap();

        let content = read_to_string(extracted.join("my-artifact")).unwrap();
        assert_eq!(content, "hello file");

        let _ = remove_dir_all(&base);
    }
}
