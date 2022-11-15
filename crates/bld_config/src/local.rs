use crate::{definitions, AuthValidation, BldLocalServerConfig, BldLocalSupervisorConfig};
use anyhow::{anyhow, Result};
use tracing::debug;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalConfig {
    pub server: BldLocalServerConfig,
    pub supervisor: BldLocalSupervisorConfig,
    pub logs: String,
    pub db: String,
    pub auth: AuthValidation,
    pub docker_url: String,
}

impl BldLocalConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let local_yaml = &yaml["local"];
        let server = BldLocalServerConfig::load(&local_yaml["server"])?;
        let supervisor = BldLocalSupervisorConfig::load(&local_yaml["supervisor"])?;
        let logs = local_yaml["logs"]
            .as_str()
            .unwrap_or(definitions::LOCAL_LOGS)
            .to_string();
        let db = local_yaml["db"]
            .as_str()
            .unwrap_or(definitions::LOCAL_DB)
            .to_string();
        let docker_url = local_yaml["docker-url"]
            .as_str()
            .unwrap_or(definitions::LOCAL_DOCKER_URL)
            .to_string();
        let auth = BldLocalConfig::auth_load(local_yaml)?;
        let instance = Self {
            server,
            supervisor,
            logs,
            db,
            auth,
            docker_url,
        };
        instance.debug_info();
        Ok(instance)
    }

    fn auth_load(yaml: &Yaml) -> Result<AuthValidation> {
        let auth_validation = match yaml["auth"]["method"].as_str() {
            Some("ldap") => AuthValidation::Ldap,
            Some("oauth2") => AuthValidation::OAuth2(
                yaml["auth"]["validation-url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("no validation url found for auth in config"))?
                    .to_string(),
            ),
            _ => AuthValidation::None,
        };
        Ok(auth_validation)
    }

    fn debug_info(&self) {
        debug!("loaded local configuration");
        debug!("server > host: {}", self.server.host);
        debug!("server > port: {}", self.server.port);
        debug!("server > pipelines: {}", self.server.pipelines);
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
        if let AuthValidation::OAuth2(url) = &self.auth {
            debug!("auth > method: oauth2");
            debug!("auth > validation-url: {}", url);
        }
    }
}

impl Default for BldLocalConfig {
    fn default() -> Self {
        Self {
            server: BldLocalServerConfig::default(),
            supervisor: BldLocalSupervisorConfig::default(),
            logs: definitions::LOCAL_LOGS.to_string(),
            db: definitions::LOCAL_DB.to_string(),
            auth: AuthValidation::None,
            docker_url: definitions::LOCAL_DOCKER_URL.to_string(),
        }
    }
}
