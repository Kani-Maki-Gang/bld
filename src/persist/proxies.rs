#![allow(dead_code)]

use crate::config::{definitions::TOOL_DIR, BldConfig};
use crate::helpers::fs::IsYaml;
use crate::path;
use crate::persist::{pipeline, PipelineFileSystemProxy};
use actix_web::web;
use anyhow::{anyhow, bail};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::fs::{create_dir_all, read_to_string, remove_file, File};
use std::io::Write;
use std::path::PathBuf;

pub struct ServerPipelineProxy {
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
}

impl ServerPipelineProxy {
    pub fn new(
        config: web::Data<BldConfig>,
        pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
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

#[derive(Default)]
pub struct LocalPipelineProxy;

impl PipelineFileSystemProxy for LocalPipelineProxy {
    fn path(&self, name: &str) -> anyhow::Result<PathBuf> {
        Ok(path![std::env::current_dir()?, TOOL_DIR, name])
    }

    fn read(&self, name: &str) -> anyhow::Result<String> {
        let path = self.path(name)?;
        Ok(read_to_string(path)?)
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
            remove_file(&path)?;
        }
        Ok(())
    }
}
