use crate::{definitions, AuthValidation, BldLocalServerConfig, BldLocalSupervisorConfig};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct BldLocalConfig {
    #[serde(default)]
    pub server: BldLocalServerConfig,

    #[serde(default)]
    pub supervisor: BldLocalSupervisorConfig,

    #[serde(default = "BldLocalConfig::default_logs")]
    pub logs: String,

    #[serde(default = "BldLocalConfig::default_db")]
    pub db: String,

    #[serde(default = "BldLocalConfig::default_docker_url")]
    pub docker_url: String,

    #[serde(default = "BldLocalConfig::default_editor")]
    pub editor: String,
}

impl BldLocalConfig {
    fn default_logs() -> String {
        definitions::LOCAL_LOGS.to_owned()
    }

    fn default_db() -> String {
        definitions::LOCAL_DB.to_owned()
    }

    fn default_docker_url() -> String {
        definitions::LOCAL_DOCKER_URL.to_owned()
    }

    fn default_editor() -> String {
        definitions::DEFAULT_EDITOR.to_owned()
    }

    pub fn debug_info(&self) {
        debug!("loaded local configuration");
        debug!("server > host: {}", self.server.host);
        debug!("server > port: {}", self.server.port);
        debug!("server > pipelines: {}", self.server.pipelines);
        if let Some(AuthValidation::OAuth2 { validation_url }) = &self.server.auth {
            debug!("auth > method: oauth2");
            debug!("auth > validation-url: {}", validation_url);
        }
        if let Some(tls) = &self.server.tls {
            debug!("server > tls > cert-chain: {}", tls.cert_chain);
            debug!("server > tls > private-key: {}", tls.private_key);
        }
        debug!("supervisor > host {}", self.supervisor.host);
        debug!("supervisor > port {}", self.supervisor.port);
        debug!("supervisor > workers {}", self.supervisor.workers);
        if let Some(tls) = &self.supervisor.tls {
            debug!("supervisor > tls > cert-chain: {}", tls.cert_chain);
            debug!("supervisor > tls > private-key: {}", tls.private_key);
        }
        debug!("logs: {}", self.logs);
        debug!("db: {}", self.db);
        debug!("docker-url: {}", self.docker_url);
    }
}

impl Default for BldLocalConfig {
    fn default() -> Self {
        Self {
            server: BldLocalServerConfig::default(),
            supervisor: BldLocalSupervisorConfig::default(),
            logs: Self::default_logs(),
            db: Self::default_db(),
            docker_url: Self::default_docker_url(),
            editor: Self::default_editor(),
        }
    }
}
