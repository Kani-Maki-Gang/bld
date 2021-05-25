use crate::config::BldServerConfig;
use crate::helpers::errors::{err_no_server_in_config, err_server_not_in_config};
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldRemoteConfig {
    pub servers: Vec<BldServerConfig>,
}

impl BldRemoteConfig {
    pub fn load(yaml: &Yaml) -> anyhow::Result<Self> {
        let servers = yaml["remote"]
            .as_vec()
            .unwrap_or(&Vec::<Yaml>::new())
            .iter()
            .map(|s| BldServerConfig::load(s))
            .filter(|s| s.is_ok())
            .map(|s| s.ok().unwrap())
            .collect();
        Ok(Self { servers })
    }

    pub fn server(&self, name: &str) -> anyhow::Result<&BldServerConfig> {
        self.servers
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(err_server_not_in_config)
    }

    pub fn nth_server(&self, i: usize) -> anyhow::Result<&BldServerConfig> {
        self.servers.get(i).ok_or_else(err_no_server_in_config)
    }

    pub fn server_or_first(&self, name: Option<&str>) -> anyhow::Result<&BldServerConfig> {
        match name {
            Some(name) => self.server(name),
            None => self.nth_server(0),
        }
    }
}

impl Default for BldRemoteConfig {
    fn default() -> Self {
        Self {
            servers: Vec::<BldServerConfig>::new(),
        }
    }
}
