mod auth;
mod command;
pub mod definitions;
mod local;
mod remote;
mod server;

pub use auth::*;
pub use command::*;
pub use local::*;
pub use remote::*;
pub use server::*;

use crate::path;
use tracing::debug;
use std::path::PathBuf;
use yaml_rust::YamlLoader;

#[derive(Debug)]
pub struct BldConfig {
    pub local: BldLocalConfig,
    pub remote: BldRemoteConfig,
}

impl BldConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = path![
            std::env::current_dir()?,
            definitions::TOOL_DIR,
            format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG)
        ];
        debug!("loading config file from: {}", &path.display());
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let yaml = YamlLoader::load_from_str(&content)?;
                let yaml = &yaml[0];

                Ok(Self {
                    local: BldLocalConfig::load(&yaml)?,
                    remote: BldRemoteConfig::load(&yaml)?,
                })
            }
            Err(_) => Ok(Self {
                local: Default::default(),
                remote: Default::default(),
            }),
        }
    }
}
