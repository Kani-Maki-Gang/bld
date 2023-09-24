use anyhow::{anyhow, bail, Result};
use bld_config::{path, BldConfig};
use bld_utils::{fs::IsYaml, sync::IntoArc};
use sea_orm::DatabaseConnection;
use std::{fmt::Write as FmtWrite, path::PathBuf, process::ExitStatus, sync::Arc};
use tokio::{
    fs::{copy, create_dir_all, read_to_string, remove_file, rename, File},
    io::AsyncWriteExt,
    process::Command,
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::database::pipeline::{self, InsertPipeline, Pipeline};

pub enum PipelineFileSystemProxy {
    Local {
        config: Arc<BldConfig>,
    },
    Server {
        config: Arc<BldConfig>,
        conn: Arc<DatabaseConnection>,
    },
}

impl Default for PipelineFileSystemProxy {
    fn default() -> Self {
        Self::Local {
            config: BldConfig::default().into_arc(),
        }
    }
}

impl PipelineFileSystemProxy {
    pub fn local(config: Arc<BldConfig>) -> Self {
        Self::Local { config }
    }

    pub fn server(config: Arc<BldConfig>, conn: Arc<DatabaseConnection>) -> Self {
        Self::Server { config, conn }
    }

    fn config(&self) -> &BldConfig {
        match self {
            Self::Server { config, .. } | Self::Local { config } => config,
        }
    }

    async fn server_path(&self, name: &str) -> Result<PathBuf> {
        let Self::Server { config, conn } = self else {
            bail!("server path isn't supported for a local proxy");
        };

        let pip = pipeline::select_by_name(conn.as_ref(), name).await?;
        Ok(path![config.server_pipelines(), format!("{}.yaml", pip.id)])
    }

    fn pipeline_path(&self, pip: &Pipeline) -> Result<PathBuf> {
        let Self::Server { config, .. } = self else {
            bail!("pipeline path isn't supported for a local proxy");
        };
        Ok(path![config.server_pipelines(), format!("{}.yaml", pip.id)])
    }

    pub async fn path(&self, name: &str) -> Result<PathBuf> {
        match self {
            Self::Local { config } => Ok(config.full_path(name)),
            Self::Server { .. } => self.server_path(name).await,
        }
    }

    async fn read_internal(&self, path: &PathBuf) -> Result<String> {
        if path.is_yaml() {
            read_to_string(path).await.map_err(|e| anyhow!(e))
        } else {
            bail!("pipeline not found")
        }
    }

    pub async fn read(&self, name: &str) -> Result<String> {
        let path = self.path(name).await?;
        self.read_internal(&path).await
    }

    pub async fn read_tmp(&self, name: &str) -> Result<String> {
        let path = self.config().tmp_full_path(name);
        self.read_internal(&path).await
    }

    async fn create_internal(&self, path: &PathBuf, content: &str, overwrite: bool) -> Result<()> {
        if path.is_yaml() && !overwrite {
            bail!("pipeline already exists");
        } else if path.is_yaml() && overwrite {
            remove_file(path).await?;
        }

        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }

        let mut handle = File::create(path).await?;
        handle.write_all(content.as_bytes()).await?;

        Ok(())
    }

    pub async fn create(&self, name: &str, content: &str, overwrite: bool) -> Result<()> {
        let local_path = self.config().full_path(name);

        if !local_path.valid_path() {
            bail!("invalid pipeline path");
        }

        if let Self::Server { conn, .. } = self {
            let response = pipeline::select_by_name(conn.as_ref(), name).await;
            if response.is_err() {
                let id = Uuid::new_v4().to_string();
                let model = InsertPipeline {
                    id,
                    name: name.to_owned(),
                };
                pipeline::insert(conn.as_ref(), model).await?;
            }
        }

        let path = self.path(name).await?;

        self.create_internal(&path, content, overwrite).await
    }

    pub async fn create_tmp(&self, name: &str, content: &str, overwrite: bool) -> Result<String> {
        let path = self.config().tmp_full_path(name);
        self.create_internal(&path, content, overwrite).await?;
        Ok(path.display().to_string())
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let path = self.path(name).await?;
        match self {
            Self::Local { .. } => {
                if path.is_yaml() {
                    remove_file(&path).await?;
                    Ok(())
                } else {
                    bail!("pipeline not found")
                }
            }
            Self::Server { .. } if !path.is_yaml() => {
                bail!("pipeline not found")
            }
            Self::Server { conn, .. } => {
                pipeline::delete_by_name(conn.as_ref(), name)
                    .await
                    .map_err(|_| anyhow!("unable to remove pipeline"))?;
                remove_file(path)
                    .await
                    .map_err(|_| anyhow!("unable to remove pipeline"))
            }
        }
    }

    pub async fn remove_tmp(&self, name: &str) -> Result<()> {
        let path = self.config().tmp_full_path(name);

        if !path.is_yaml() {
            bail!("pipeline not found");
        }

        remove_file(path).await?;
        Ok(())
    }

    pub async fn copy(&self, source: &str, target: &str) -> Result<()> {
        match self {
            Self::Local { .. } => {
                let source_path = self.path(source).await?;
                if !source_path.is_yaml() {
                    bail!("invalid source pipeline path");
                }
                let target_path = self.path(target).await?;
                if !target_path.valid_path() {
                    bail!("invalid target pipeline path");
                }
                if target_path.is_yaml() {
                    bail!("target pipeline already exists");
                }
                if let Some(parent) = target_path.parent() {
                    create_dir_all(parent).await?;
                }
                copy(source_path, target_path).await?;
                Ok(())
            }
            Self::Server { .. } => {
                let content = self.read(source).await?;
                self.create(target, &content, false).await
            }
        }
    }

    pub async fn mv(&self, source: &str, target: &str) -> Result<()> {
        let source_path = self.path(source).await?;
        if !source_path.is_yaml() {
            bail!("invalid source pipeline path");
        }

        let target_path = self.config().full_path(target);
        if !target_path.valid_path() {
            bail!("invalid target pipeline path");
        }

        match self {
            Self::Local { .. } => {
                if target_path.is_yaml() {
                    bail!("target pipeline already exist");
                }
                if let Some(parent) = target_path.parent() {
                    create_dir_all(parent).await?;
                }
                rename(source_path, target_path).await?;
                Ok(())
            }
            Self::Server { conn, .. } => {
                let conn = conn.as_ref();
                let source_pipeline = pipeline::select_by_name(conn, source).await?;
                if pipeline::select_by_name(conn, target).await.is_ok() {
                    bail!("target pipeline already exist");
                };
                pipeline::update_name(conn, &source_pipeline.id, target).await
            }
        }
    }

    pub async fn list(&self) -> Result<Vec<String>> {
        match self {
            Self::Local { config, .. } => {
                let root_dir = format!("{}/", config.root_dir);
                let mut entries: Vec<String> = WalkDir::new(&root_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_yaml())
                    .map(|e| {
                        let mut entry = e.path().display().to_string();
                        entry = entry.replace(&root_dir, "");
                        entry
                    })
                    .collect();
                entries.sort();
                Ok(entries)
            }
            Self::Server { conn, .. } => {
                let pips = pipeline::select_all(conn.as_ref())
                    .await?
                    .iter()
                    .filter(|p| {
                        self.pipeline_path(p)
                            .as_ref()
                            .map(|p| p.is_yaml())
                            .unwrap_or_default()
                    })
                    .map(|p| p.name.clone())
                    .collect();
                Ok(pips)
            }
        }
    }

    async fn edit_internal(&self, path: &PathBuf, check_path: bool) -> Result<()> {
        let Self::Local { config } = self else {
            bail!("server pipelines dont support direct editing");
        };

        if check_path && !path.is_yaml() {
            bail!("pipeline not found");
        }

        let mut editor = Command::new("/bin/bash");
        editor.args(["-c", &format!("{} {}", config.local.editor, path.display())]);

        let status = editor.status().await?;
        if !ExitStatus::success(&status) {
            let mut error = String::new();
            let output = editor.output().await?;
            writeln!(error, "editor process finished with {}", status)?;
            write!(error, "{}", String::from_utf8_lossy(&output.stderr))?;
            bail!(error);
        }
        Ok(())
    }

    pub async fn edit(&self, name: &str) -> Result<()> {
        let path = self.path(name).await?;
        self.edit_internal(&path, true).await
    }

    pub async fn edit_tmp(&self, name: &str) -> Result<()> {
        let path = self.config().tmp_full_path(name);
        self.edit_internal(&path, true).await
    }

    pub async fn edit_config(&self) -> Result<()> {
        self.edit_internal(&self.config().config_full_path(), false)
            .await
    }
}
