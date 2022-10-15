use crate::{err_no_server_in_config, err_server_not_in_config, BldRemoteServerConfig};
use anyhow::{bail, Result};
use yaml_rust::Yaml;

#[derive(Debug, Default)]
pub struct BldRemoteConfig {
    pub servers: Vec<BldRemoteServerConfig>,
}

impl BldRemoteConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let servers = yaml["remote"]
            .as_vec()
            .unwrap_or(&Vec::<Yaml>::new())
            .iter()
            .map(BldRemoteServerConfig::load)
            .filter(|s| s.is_ok())
            .map(|s| s.ok().unwrap())
            .collect();
        Ok(Self { servers })
    }

    pub fn server(&self, name: &str) -> Result<&BldRemoteServerConfig> {
        self.servers
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(err_server_not_in_config)
    }

    pub fn nth_server(&self, i: usize) -> Result<&BldRemoteServerConfig> {
        self.servers.get(i).ok_or_else(err_no_server_in_config)
    }

    pub fn server_or_first(&self, name: Option<&str>) -> Result<&BldRemoteServerConfig> {
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
            return match self.servers.iter().find(|s| &s.name == name) {
                Some(srv) => Ok(srv),
                None => bail!("could not parse auth settings for server"),
            };
        }
        Ok(server)
    }
}
