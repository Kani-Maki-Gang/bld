use crate::database::pipeline;
use anyhow::{anyhow, bail};
use bld_config::{definitions::TOOL_DIR, path, BldConfig};
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::{
    fs::{create_dir_all, read_to_string, remove_file, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
};

pub enum PipelineFileSystemProxy {
    Local,
    Server {
        config: Arc<BldConfig>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    },
}

impl PipelineFileSystemProxy {
    pub fn path(&self, name: &str) -> anyhow::Result<PathBuf> {
        match self {
            Self::Local => Ok(path![std::env::current_dir()?, TOOL_DIR, name]),
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

    pub fn read(&self, name: &str) -> anyhow::Result<String> {
        match self {
            Self::Local => {
                let path = self.path(name)?;
                Ok(read_to_string(path)?)
            }
            Self::Server { config: _, pool: _ } => {
                let path = self.path(name)?;
                if path.is_yaml() {
                    return Ok(read_to_string(path)?);
                }
                Err(anyhow!("pipeline not found"))
            }
        }
    }

    pub fn create(&self, name: &str, content: &str) -> anyhow::Result<()> {
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

    pub fn remove(&self, name: &str) -> anyhow::Result<()> {
        match self {
            Self::Local => {
                let path = self.path(name)?;
                if path.is_yaml() {
                    remove_file(&path)?;
                }
                Ok(())
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
