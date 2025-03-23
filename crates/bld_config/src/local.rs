use std::collections::HashMap;

use crate::{
    BldLocalServerConfig, BldLocalSupervisorConfig, DockerUrl, RegistryConfig, definitions,
    ssh::SshConfig,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tokio")]
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct BldLocalConfig {
    #[serde(default)]
    pub server: BldLocalServerConfig,

    #[serde(default)]
    pub supervisor: BldLocalSupervisorConfig,

    #[serde(default)]
    pub docker_url: DockerUrl,

    #[serde(default = "BldLocalConfig::default_editor")]
    pub editor: String,

    #[serde(default)]
    pub ssh: HashMap<String, SshConfig>,

    #[serde(default)]
    pub registries: HashMap<String, RegistryConfig>,
}

impl BldLocalConfig {
    fn default_editor() -> String {
        definitions::DEFAULT_EDITOR.to_owned()
    }

    #[cfg(feature = "tokio")]
    pub fn debug_info(&self) {
        use crate::{Auth, DockerUrlEntry, SshUserAuth};

        debug!("loaded local configuration");
        debug!("server > host: {}", self.server.host);
        debug!("server > port: {}", self.server.port);
        debug!("server > pipelines: {}", self.server.pipelines);
        debug!("logs: {}", self.server.logs);
        debug!("db: {:?}", self.server.db);
        if let Some(Auth::OpenId(openid)) = &self.server.auth {
            debug!("auth > method: openid");
            debug!("auth > issuer_url: {:?}", openid.issuer_url);
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
        for (key, config) in &self.ssh {
            debug!("ssh > {key} > host: {}", config.host);
            debug!("ssh > {key} > port: {}", config.port);
            debug!("ssh > {key} > user: {}", config.user);
            match &config.userauth {
                SshUserAuth::Keys {
                    public_key,
                    private_key,
                } => {
                    debug!("ssh > {key} > userauth > type: keys");
                    debug!("ssh > {key} > userauth > public_key: {public_key:?}");
                    debug!("ssh > {key} > userauth > private_key: {private_key}");
                }
                SshUserAuth::Password { .. } => {
                    debug!("ssh > {key} > userauth > type: password");
                    debug!("ssh > {key} > userauth > password: ********");
                }
                SshUserAuth::Agent => {
                    debug!("ssh > {key} > userauth > type: agent");
                }
            }
        }
        match &self.docker_url {
            DockerUrl::Single(url) => debug!("docker_url: {url}"),
            DockerUrl::Multiple(urls) => {
                for (key, value) in urls {
                    match value {
                        DockerUrlEntry::Url(value) => debug!("docker_url > {key}: {value}"),
                        DockerUrlEntry::UrlWithDefault { url, default } => {
                            debug!("docker_url > {key}: {url} ({default})");
                        }
                    }
                }
            }
        }
    }
}

impl Default for BldLocalConfig {
    fn default() -> Self {
        Self {
            server: Default::default(),
            supervisor: Default::default(),
            docker_url: Default::default(),
            editor: Self::default_editor(),
            ssh: Default::default(),
            registries: Default::default(),
        }
    }
}
