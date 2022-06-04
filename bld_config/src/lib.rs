mod auth;
pub mod definitions;
mod errors;
mod local;
mod remote;
mod server;
mod path;

pub use auth::*;
pub use errors::*;
pub use local::*;
pub use remote::*;
pub use server::*;
pub use path::*;

use std::path::PathBuf;
use tracing::debug;
use yaml_rust::YamlLoader;

#[derive(Debug, Default)]
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
                    local: BldLocalConfig::load(yaml)?,
                    remote: BldRemoteConfig::load(yaml)?,
                })
            }
            Err(_) => Ok(BldConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
