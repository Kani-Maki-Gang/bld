use crate::definitions;
use crate::{Auth, BldTlsConfig, OAuth2Info};
use anyhow::{anyhow, Result};
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

    pub fn http_protocol(&self) -> String {
        if self.tls.is_some() {
            "https".to_string()
        } else {
            "http".to_string()
        }
    }

    pub fn ws_protocol(&self) -> String {
        if self.tls.is_some() {
            "https".to_string()
        } else {
            "http".to_string()
        }
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

#[derive(Debug, Clone)]
pub struct BldRemoteServerConfig {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub tls: bool,
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
        let tls = yaml["tls"].as_bool().unwrap_or(false);
        let protocol = Self::http_protocol_internal(tls);
        let auth = match yaml["auth"]["method"].as_str() {
            Some("ldap") => Auth::Ldap,
            Some("oauth2") => {
                Auth::OAuth2(OAuth2Info::load(&host, port, &protocol, &yaml["auth"])?)
            }
            _ => Auth::None,
        };
        let same_auth_as = yaml["same-auth-as"].as_str().map(|s| s.to_string());
        Ok(Self {
            name,
            host,
            port,
            tls,
            auth,
            same_auth_as,
        })
    }

    fn http_protocol_internal(tls: bool) -> String {
        if tls {
            "https".to_string()
        } else {
            "http".to_string()
        }
    }

    /// Checks the value of the tls field and returns the appropriate form
    /// of the http protocol to be used, either http or https.
    pub fn http_protocol(&self) -> String {
        Self::http_protocol_internal(self.tls)
    }

    /// Checks the value of the tls field and returns the appropriate form
    /// of th ws protocol to be used, either ws or wss.
    pub fn ws_protocol(&self) -> String {
        if self.tls {
            "wss".to_string()
        } else {
            "ws".to_string()
        }
    }
}
