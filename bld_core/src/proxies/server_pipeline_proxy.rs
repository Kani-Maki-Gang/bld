#![allow(dead_code)]

use crate::database::pipeline;
use crate::proxies::PipelineFileSystemProxy;
use anyhow::{anyhow, bail};
use bld_config::{path, BldConfig};
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::fs::{create_dir_all, read_to_string, remove_file, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ServerPipelineProxy {
    config: Arc<BldConfig>,
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

impl ServerPipelineProxy {
    pub fn new(
        config: Arc<BldConfig>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    ) -> Self {
        Self { config, pool }
    }
}

impl PipelineFileSystemProxy for ServerPipelineProxy {
    fn path(&self, name: &str) -> anyhow::Result<PathBuf> {
        let conn = self.pool.get()?;
        let pip = pipeline::select_by_name(&conn, name)?;
        Ok(path![
            &self.config.local.server_pipelines,
            format!("{}.yaml", pip.id)
        ])
    }

    fn read(&self, name: &str) -> anyhow::Result<String> {
        let path = self.path(name)?;
        if path.is_yaml() {
            return Ok(read_to_string(path)?);
        }
        Err(anyhow!("pipeline not found"))
    }

    fn create(&self, name: &str, content: &str) -> anyhow::Result<()> {
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

    fn remove(&self, name: &str) -> anyhow::Result<()> {
        let path = self.path(name)?;
        if path.is_yaml() {
            let conn = self.pool.get()?;
            pipeline::delete_by_name(&conn, name)
                .and_then(|_| remove_file(path).map_err(|e| anyhow!(e)))
                .map_err(|_| anyhow!("unable to remove pipeline"))
        } else {
            bail!("pipeline not found")
        }
    }
}
