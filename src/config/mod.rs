mod cli;
mod exec;
mod local;
mod remote;
mod server;

pub use cli::*;
pub use exec::*;
pub use local::*;
pub use remote::*;
pub use server::*;

use crate::definitions;
use std::io::{self, Error, ErrorKind};
use yaml_rust::YamlLoader;

#[derive(Debug)]
pub struct BldConfig {
    pub local: BldLocalConfig,
    pub remote: BldRemoteConfig,
}

impl BldConfig {
    pub fn load() -> io::Result<Self> {
        let mut path = std::env::current_dir()?;
        path.push(definitions::TOOL_DIR);
        path.push(format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG));

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let yaml = match YamlLoader::load_from_str(&content) {
                    Ok(yaml) => yaml,
                    Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
                };
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
