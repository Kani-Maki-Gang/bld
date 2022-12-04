mod auth;
pub mod definitions;
mod local;
mod path;
mod server;
mod supervisor;
mod tls;

pub use auth::*;
pub use local::*;
pub use path::*;
pub use server::*;
pub use supervisor::*;
pub use tls::*;

use anyhow::{anyhow, bail, Error, Result};
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    pub local: BldLocalConfig,

    #[serde(default)]
    pub remote: Vec<BldRemoteServerConfig>,
}

impl BldConfig {
    pub fn load() -> Result<Self> {
        let path = path![
            std::env::current_dir()?,
            definitions::TOOL_DIR,
            format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG)
        ];
        debug!("loading config file from: {}", &path.display());
        serde_yaml::from_str(&read_to_string(&path)?).map_err(|e| anyhow!(e))
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

    pub fn server_or_first(&self, name: Option<&String>) -> Result<&BldRemoteServerConfig> {
        match name {
            Some(name) => self.server(name),
            None => self.nth_server(0),
        }
    }

    pub fn same_auth_as<'a>(
        &'a self,
        server: &'a BldRemoteServerConfig,
    ) -> Result<&'a BldRemoteServerConfig> {
        if let Some(name) = &server.same_auth_as {
            return match self.remote.iter().find(|s| &s.name == name) {
                Some(srv) => Ok(srv),
                None => bail!("could not parse auth settings for server"),
            };
        }
        Ok(server)
    }
}
