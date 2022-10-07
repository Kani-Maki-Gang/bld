use crate::definitions;
use crate::{Auth, OAuth2Info, BldTlsConfig};
use anyhow::{anyhow, Result};
use async_raft::NodeId;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalServerConfig {
    pub host: String,
    pub port: i64,
    pub tls: Option<BldTlsConfig>,
    pub pipelines: String,
}

impl BldLocalServerConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let host = yaml["host"]
            .as_str()
            .unwrap_or(definitions::LOCAL_SERVER_HOST)
            .to_string();
        let port = yaml["port"]
            .as_i64()
            .unwrap_or(definitions::LOCAL_SERVER_PORT);
        let tls = BldTlsConfig::load(&yaml["tls"])?;
        let pipelines = yaml["pipelines"]
            .as_str()
            .unwrap_or(definitions::LOCAL_SERVER_PIPELINES)
            .to_string();
        Ok(Self {
            host,
            port,
            tls,
            pipelines,
        })
    }
}

impl Default for BldLocalServerConfig {
    fn default() -> Self {
        Self {
            host: definitions::LOCAL_SERVER_HOST.to_string(),
            port: definitions::LOCAL_SERVER_PORT,
            tls: None,
            pipelines: definitions::LOCAL_SERVER_PIPELINES.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct BldRemoteServerConfig {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub tls: bool,
    pub node_id: Option<NodeId>,
    pub auth: Auth,
    pub same_auth_as: Option<String>,
}

impl BldRemoteServerConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
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
        let tls = yaml["tls"]
            .as_bool()
            .unwrap_or(false);
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
            tls,
            node_id,
            auth,
            same_auth_as,
        })
    }
}
