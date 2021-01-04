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
        let servers = yaml["remote"]
            .as_vec()
            .or(Some(&Vec::new()))
            .unwrap()
            .iter()
            .map(|s| BldServerConfig::load(s))
            .filter(|s| s.is_ok())
            .map(|s| s.ok().unwrap())
            .collect();
        Ok(Self { servers })
    }
}
