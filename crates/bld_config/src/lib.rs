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

pub fn err_server_not_in_config() -> Error {
    anyhow!("server not found in config")
}

pub fn err_no_server_in_config() -> Error {
    anyhow!("no server found in config")
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BldConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub path: String,

    #[serde(default)]
    pub local: BldLocalConfig,

    #[serde(default)]
    pub remote: Vec<BldRemoteServerConfig>,
}

impl BldConfig {
    pub fn path() -> Result<PathBuf> {
        Ok(path![
            current_dir()?,
            definitions::TOOL_DIR,
            format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG)
        ])
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        debug!("loading config file from: {}", &path.display());

        let mut instance: Self =
            serde_yaml::from_str(&read_to_string(&path)?).map_err(|e| anyhow!(e))?;

        instance.path = path
            .into_os_string()
            .into_string()
            .map_err(|_| anyhow!("unable to get text reprasentation of config path"))?;

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
}
