mod auth;
pub mod definitions;
mod local;
mod path;
mod server;
mod supervisor;
mod tls;

pub use auth::*;
pub use local::*;
use openidconnect::core::CoreClient;
pub use path::*;
pub use server::*;
pub use supervisor::*;
pub use tls::*;

use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use std::env::current_dir;
use std::fs::read_to_string;
use std::path::PathBuf;
use tracing::debug;

use crate::definitions::{LOCAL_MACHINE_TMP_DIR, TOOL_DEFAULT_CONFIG_FILE};

pub fn err_server_not_in_config() -> Error {
    anyhow!("server not found in config")
}

pub fn err_no_server_in_config() -> Error {
    anyhow!("no server found in config")
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BldConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub root_dir: String,

    #[serde(skip_serializing, skip_deserializing)]
    pub project_dir: String,

    #[serde(default)]
    pub local: BldLocalConfig,

    #[serde(default)]
    pub remote: Vec<BldRemoteServerConfig>,
}

impl BldConfig {
    pub fn path() -> Result<PathBuf> {
        let mut current = current_dir()?;
        loop {
            let cfg_file = path![
                &current,
                definitions::TOOL_DIR,
                format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG)
            ];

            if !cfg_file.exists() {
                current = current
                    .parent()
                    .map(|p| p.to_path_buf())
                    .ok_or_else(|| anyhow!(".bld directory not found"))?;
                continue;
            }

            return Ok(cfg_file);
        }
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        let root_dir = path
            .parent()
            .ok_or_else(|| anyhow!("unable to resolve config path"))?;

        let project_dir = root_dir
            .parent()
            .ok_or_else(|| anyhow!("unable to resolve project path"))?;

        debug!("loading config file from: {}", &path.display());

        let mut instance: Self =
            serde_yaml::from_str(&read_to_string(&path)?).map_err(|e| anyhow!(e))?;

        instance.root_dir = root_dir
            .to_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("unable to construct config path"))?;

        instance.project_dir = project_dir
            .to_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("unable to construct project path"))?;

        instance.local.debug_info();
        Ok(instance)
    }

    pub fn server(&self, name: &str) -> Result<&BldRemoteServerConfig> {
        self.remote
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(err_server_not_in_config)
    }

    pub fn nth_server(&self, i: usize) -> Result<&BldRemoteServerConfig> {
        self.remote.get(i).ok_or_else(err_no_server_in_config)
    }

    pub async fn openid_core_client(&self) -> Result<Option<CoreClient>> {
        if let Some(auth) = &self.local.server.auth {
            auth.core_client().await.map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn server_pipelines(&self) -> PathBuf {
        path![&self.root_dir, &self.local.server.pipelines]
    }

    pub fn config_full_path(&self) -> PathBuf {
        path![&self.root_dir, TOOL_DEFAULT_CONFIG_FILE]
    }

    pub fn full_path(&self, name: &str) -> PathBuf {
        path![&self.root_dir, name]
    }

    pub fn tmp_full_path(&self, name: &str) -> PathBuf {
        path![&self.root_dir, LOCAL_MACHINE_TMP_DIR, name]
    }
}
