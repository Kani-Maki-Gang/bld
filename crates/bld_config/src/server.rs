use crate::definitions::REMOTE_SERVER_OAUTH2;
use crate::{definitions, path};
use crate::{Auth, BldTlsConfig};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct BldLocalServerConfig {
    #[serde(default = "BldLocalServerConfig::default_host")]
    pub host: String,

    #[serde(default = "BldLocalServerConfig::default_port")]
    pub port: i64,

    pub tls: Option<BldTlsConfig>,

    #[serde(default = "BldLocalServerConfig::default_pipelines")]
    pub pipelines: String,
}

impl BldLocalServerConfig {
    fn default_host() -> String {
        definitions::LOCAL_SERVER_HOST.to_owned()
    }

    fn default_port() -> i64 {
        definitions::LOCAL_SERVER_PORT
    }

    fn default_pipelines() -> String {
        definitions::LOCAL_SERVER_PIPELINES.to_owned()
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
            host: Self::default_host(),
            port: Self::default_port(),
            tls: None,
            pipelines: Self::default_pipelines(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BldRemoteServerConfig {
    #[serde(rename(serialize = "server", deserialize = "server"))]
    pub name: String,

    pub host: String,
    pub port: i64,

    #[serde(default)]
    pub tls: bool,

    #[serde(default)]
    pub auth: Option<Auth>,

    #[serde(default)]
    pub same_auth_as: Option<String>,
}

impl BldRemoteServerConfig {
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

    pub fn bearer(&self) -> Result<String> {
        let path = path![REMOTE_SERVER_OAUTH2, &self.name];
        read_to_string(path).map_err(|e| anyhow!(e))
    }
}
