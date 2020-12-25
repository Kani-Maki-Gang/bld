mod cli;
pub mod definitions;
mod exec;
mod local;
mod remote;
mod server;

pub use cli::*;
pub use exec::*;
pub use local::*;
pub use remote::*;
pub use server::*;

use crate::path;
use crate::types::Result;
use std::path::PathBuf;
use yaml_rust::YamlLoader;

#[derive(Debug)]
pub struct BldConfig {
    pub local: BldLocalConfig,
    pub remote: BldRemoteConfig,
}

impl BldConfig {
    pub fn load() -> Result<Self> {
        let path = path![
            std::env::current_dir()?,
            definitions::TOOL_DIR,
            format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG)
        ];
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
                local: BldLocalConfig::default(),
                remote: BldRemoteConfig::default(),
            }),
        }
    }
}
