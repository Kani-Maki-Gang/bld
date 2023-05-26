use crate::database::pipeline;
use anyhow::{anyhow, bail, Result};
use bld_config::{definitions::TOOL_DIR, path, BldConfig};
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::env::current_dir;
use std::fs::{create_dir_all, read_to_string, remove_file, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

pub enum PipelineFileSystemProxy {
    Local,
    Server {
        config: Arc<BldConfig>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    },
}

impl Default for PipelineFileSystemProxy {
    fn default() -> Self {
        Self::Local
    }
}

impl PipelineFileSystemProxy {
    pub fn path(&self, name: &str) -> Result<PathBuf> {
        match self {
            Self::Local => Ok(path![current_dir()?, TOOL_DIR, name]),
            Self::Server { config, pool } => {
                let mut conn = pool.get()?;
                let pip = pipeline::select_by_name(&mut conn, name)?;
                Ok(path![
                    &config.local.server.pipelines,
                    format!("{}.yaml", pip.id)
                ])
            }
        }
    }

    pub fn read(&self, name: &str) -> Result<String> {
        let path = self.path(name)?;

        match self {
            Self::Local | Self::Server { config: _, pool: _ } if path.is_yaml() => {
                read_to_string(path).map_err(|e| anyhow!(e))
            }
            _ => bail!("pipeline not found"),
        }
    }

    pub fn create(&self, name: &str, content: &str) -> Result<()> {
        match self {
            Self::Local => {
                let path = self.path(name)?;
                if path.is_yaml() {
                    remove_file(&path)?;
                } else if let Some(parent) = path.parent() {
                    create_dir_all(parent)?;
                }
                let mut handle = File::create(&path)?;
                handle.write_all(content.as_bytes())?;
                Ok(())
            }
            Self::Server { config: _, pool: _ } => {
                let path = self.path(name)?;
                if path.is_yaml() {
                    remove_file(&path)?;
                } else if let Some(parent) = path.parent() {
                    create_dir_all(parent)?;
                }
                let mut handle = File::create(&path)?;
                handle.write_all(content.as_bytes())?;
                Ok(())
            }
        }
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        match self {
            Self::Local => {
                let path = self.path(name)?;
                if path.is_yaml() {
                    remove_file(&path)?;
                    Ok(())
                } else {
                    bail!("pipeline not found")
                }
            }
            Self::Server { config: _, pool } => {
                let path = self.path(name)?;
                if path.is_yaml() {
                    let mut conn = pool.get()?;
                    pipeline::delete_by_name(&mut conn, name)
                        .and_then(|_| remove_file(path).map_err(|e| anyhow!(e)))
                        .map_err(|_| anyhow!("unable to remove pipeline"))
                } else {
                    bail!("pipeline not found")
                }
            }
        }
    }
}
