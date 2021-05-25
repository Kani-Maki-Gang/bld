use anyhow::anyhow;
use crate::config::{Auth, OAuth2Info};
use async_raft::NodeId;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldServerConfig {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub node_id: Option<NodeId>,
    pub auth: Auth,
    pub same_auth_as: Option<String>,
}

impl BldServerConfig {
    pub fn load(yaml: &Yaml) -> anyhow::Result<Self> {
        let name = yaml["server"]
            .as_str()
            .ok_or_else(|| anyhow!("Server entry must have a name"))?
            .to_string();
        let host = yaml["host"]
            .as_str()
            .ok_or_else(|| anyhow!("Server entry must define a host address"))?
            .to_string();
        let port = yaml["port"]
            .as_i64()
            .ok_or_else(|| anyhow!("Server entry must define a port"))?;
        let node_id = yaml["node-id"].as_i64().map(|n| n as NodeId);
        let auth = match yaml["auth"]["method"].as_str() {
            Some("ldap") => Auth::Ldap,
            Some("oauth2") => Auth::OAuth2(OAuth2Info::load(&host, port, &yaml["auth"])?),
            _ => Auth::None,
        };
        let same_auth_as = yaml["same-auth-as"].as_str().map(|s| s.to_string());
        Ok(Self {
            name,
            host,
            port,
            node_id,
            auth,
            same_auth_as,
        })
    }
}
