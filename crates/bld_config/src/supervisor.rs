use crate::BldTlsConfig;
use crate::definitions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BldLocalSupervisorConfig {
    #[serde(default = "BldLocalSupervisorConfig::default_host")]
    pub host: String,

    #[serde(default = "BldLocalSupervisorConfig::default_port")]
    pub port: i64,

    pub tls: Option<BldTlsConfig>,

    #[serde(default = "BldLocalSupervisorConfig::default_workers")]
    pub workers: i64,
}

impl BldLocalSupervisorConfig {
    fn default_host() -> String {
        definitions::LOCAL_SUPERVISOR_HOST.to_owned()
    }

    fn default_port() -> i64 {
        definitions::LOCAL_SUPERVISOR_PORT
    }

    fn default_workers() -> i64 {
        definitions::LOCAL_SUPERVISOR_WORKERS
    }

    fn http_protocol(&self) -> String {
        if self.tls.is_some() {
            "https".to_string()
        } else {
            "http".to_string()
        }
    }

    pub fn base_url_http(&self) -> String {
        format!("{}://{}:{}", self.http_protocol(), self.host, self.port)
    }

    fn ws_protocol(&self) -> String {
        if self.tls.is_some() {
            "wss".to_string()
        } else {
            "ws".to_string()
        }
    }

    pub fn base_url_ws(&self) -> String {
        format!("{}://{}:{}", self.ws_protocol(), self.host, self.port)
    }
}

impl Default for BldLocalSupervisorConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            tls: None,
            workers: Self::default_workers(),
        }
    }
}
