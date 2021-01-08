use crate::config::{Auth, OAuth2Info};
use crate::types::Result;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldServerConfig {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub auth: Auth,
}

impl BldServerConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let name = yaml["server"]
            .as_str()
            .ok_or("Server entry must have a name")?
            .to_string();
        let host = yaml["host"]
            .as_str()
            .ok_or("Server entry must define a host address")?
            .to_string();
        let port = yaml["port"]
            .as_i64()
            .ok_or("Server entry must define a port")?;
        let auth = match yaml["auth"]["method"].as_str() {
            Some("ldap") => Auth::Ldap,
            Some("oauth2") => Auth::OAuth2(OAuth2Info::load(&host, port, &yaml["auth"])?),
            _ => Auth::None,
        };
        Ok(Self {
            name,
            host,
            port,
            auth,
        })
    }
}
