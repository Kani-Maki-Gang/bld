use crate::config::BldServerConfig;
use crate::types::Result;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldRemoteConfig {
    pub servers: Vec<BldServerConfig>,
}

impl BldRemoteConfig {
    pub fn default() -> Self {
        Self {
            servers: Vec::<BldServerConfig>::new(),
        }
    }

    pub fn load(yaml: &Yaml) -> Result<Self> {
        let mut servers = Vec::<BldServerConfig>::new();

        if let Some(yaml) = yaml["remote"].as_vec() {
            for entry in yaml.iter() {
                servers.push(BldServerConfig::load(&entry)?);
            }
        }

        Ok(Self { servers })
    }
}
