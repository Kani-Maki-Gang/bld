use std::collections::HashMap;

use crate::{definitions, ssh::SshConfig, Auth, BldLocalServerConfig, BldLocalSupervisorConfig};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct BldLocalConfig {
    #[serde(default)]
    pub server: BldLocalServerConfig,

    #[serde(default)]
    pub supervisor: BldLocalSupervisorConfig,

    #[serde(default = "BldLocalConfig::default_docker_url")]
    pub docker_url: String,

    #[serde(default = "BldLocalConfig::default_editor")]
    pub editor: String,

    #[serde(default)]
    pub ssh: HashMap<String, SshConfig>,
}

impl BldLocalConfig {
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
        debug!("logs: {}", self.server.logs);
        debug!("db: {:?}", self.server.db);
        if let Some(Auth::OpenId(openid)) = &self.server.auth {
            debug!("auth > method: openid");
            debug!("auth > issuer_url: {:?}", openid.issuer_url);
            debug!("auth > redirect_url: {:?}", openid.redirect_url);
            debug!("auth > client_id: {:?}", openid.client_id);
            debug!("auth > client_secret: ********");

            let scopes = openid
                .scopes
                .iter()
                .map(|x| x.to_string())
                .reduce(|acc, n| acc + "," + &n)
                .unwrap_or_default();
            debug!("auth > scopes: {}", scopes);
            debug!("auth > user_property: {}", openid.user_property);
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
        debug!("docker-url: {}", self.docker_url);
    }
}

impl Default for BldLocalConfig {
    fn default() -> Self {
        Self {
            server: BldLocalServerConfig::default(),
            supervisor: BldLocalSupervisorConfig::default(),
            docker_url: Self::default_docker_url(),
            editor: Self::default_editor(),
            ssh: Default::default(),
        }
    }
}
