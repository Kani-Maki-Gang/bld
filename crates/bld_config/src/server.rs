use crate::{Auth, BldTlsConfig, definitions};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BldLocalServerConfig {
    #[serde(default = "BldLocalServerConfig::default_host")]
    pub host: String,

    #[serde(default = "BldLocalServerConfig::default_port")]
    pub port: i64,

    pub auth: Option<Auth>,

    pub tls: Option<BldTlsConfig>,

    #[serde(default = "BldLocalServerConfig::default_pipelines")]
    pub pipelines: String,

    #[serde(default = "BldLocalServerConfig::default_logs")]
    pub logs: String,

    pub db: Option<String>,
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

    fn default_logs() -> String {
        definitions::LOCAL_LOGS.to_owned()
    }

    /// Checks the value of the tls field and returns the appropriate form
    /// of the http protocol to be used, either http or https.
    fn http_protocol(&self) -> String {
        if self.tls.is_some() {
            "https".to_string()
        } else {
            "http".to_string()
        }
    }

    // Returns the base url for the server using the http or https protocol
    // depending on the server's tls options.
    pub fn base_url_http(&self) -> String {
        format!("{}://{}:{}", self.http_protocol(), self.host, self.port)
    }

    /// Checks the value of the tls field and returns the appropriate form
    /// of th ws protocol to be used, either ws or wss.
    fn ws_protocol(&self) -> String {
        if self.tls.is_some() {
            "wss".to_string()
        } else {
            "ws".to_string()
        }
    }

    // Returns the base url for the server using the ws or wss protocol
    // depending on the server's tls options.
    pub fn base_url_ws(&self) -> String {
        format!("{}://{}:{}", self.ws_protocol(), self.host, self.port)
    }
}

impl Default for BldLocalServerConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            tls: None,
            auth: None,
            pipelines: Self::default_pipelines(),
            logs: Self::default_logs(),
            db: None,
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
}

impl BldRemoteServerConfig {
    /// Checks the value of the tls field and returns the appropriate form
    /// of the http protocol to be used, either http or https.
    fn http_protocol(&self) -> String {
        if self.tls {
            "https".to_string()
        } else {
            "http".to_string()
        }
    }

    // Returns the base url for the server using the http or https protocol
    // depending on the server's tls options.
    pub fn base_url_http(&self) -> String {
        format!("{}://{}:{}", self.http_protocol(), self.host, self.port)
    }

    /// Checks the value of the tls field and returns the appropriate form
    /// of th ws protocol to be used, either ws or wss.
    fn ws_protocol(&self) -> String {
        if self.tls {
            "wss".to_string()
        } else {
            "ws".to_string()
        }
    }

    // Returns the base url for the server using the ws or wss protocol
    // depending on the server's tls options.
    pub fn base_url_ws(&self) -> String {
        format!("{}://{}:{}", self.ws_protocol(), self.host, self.port)
    }
}
