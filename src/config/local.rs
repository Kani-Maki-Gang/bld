use crate::config::definitions;
use crate::config::AuthValidation;
use anyhow::anyhow;
use async_raft::NodeId;
use tracing::debug;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalConfig {
    pub ha_mode: bool,
    pub node_id: Option<NodeId>,
    pub host: String,
    pub port: i64,
    pub logs: String,
    pub db: String,
    pub server_pipelines: String,
    pub auth: AuthValidation,
    pub docker_url: String,
}

impl BldLocalConfig {
    pub fn load(yaml: &Yaml) -> anyhow::Result<Self> {
        let local_yaml = &yaml["local"];
        let ha_mode = local_yaml["ha-mode"]
            .as_bool()
            .unwrap_or(definitions::LOCAL_HA_MODE);
        let node_id = local_yaml["node-id"].as_i64().map(|n| n as NodeId);
        let host = local_yaml["host"]
            .as_str()
            .unwrap_or(definitions::LOCAL_SERVER_HOST)
            .to_string();
        let port = local_yaml["port"]
            .as_i64()
            .unwrap_or(definitions::LOCAL_SERVER_PORT);
        let logs = local_yaml["logs"]
            .as_str()
            .unwrap_or(definitions::LOCAL_LOGS)
            .to_string();
        let db = local_yaml["db"]
            .as_str()
            .unwrap_or(definitions::LOCAL_DB)
            .to_string();
        let server_pipelines = local_yaml["server-pipelines"]
            .as_str()
            .unwrap_or(definitions::LOCAL_SERVER_PIPELINES)
            .to_string();
        let docker_url = local_yaml["docker-url"]
            .as_str()
            .unwrap_or(definitions::LOCAL_DOCKER_URL)
            .to_string();
        let auth = BldLocalConfig::auth_load(local_yaml)?;
        let instance = Self {
            ha_mode,
            node_id,
            host,
            port,
            logs,
            db,
            server_pipelines,
            auth,
            docker_url,
        };
        instance.debug_info();
        Ok(instance)
    }

    fn auth_load(yaml: &Yaml) -> anyhow::Result<AuthValidation> {
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
        debug!("ha-mode: {}", self.ha_mode);
        debug!("node-id: {:?}", self.node_id);
        debug!("host: {}", self.host);
        debug!("port: {}", self.port);
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
            ha_mode: definitions::LOCAL_HA_MODE,
            node_id: None,
            host: definitions::LOCAL_SERVER_HOST.to_string(),
            port: definitions::LOCAL_SERVER_PORT,
            logs: definitions::LOCAL_LOGS.to_string(),
            db: definitions::LOCAL_DB.to_string(),
            server_pipelines: definitions::LOCAL_SERVER_PIPELINES.to_string(),
            auth: AuthValidation::None,
            docker_url: definitions::LOCAL_DOCKER_URL.to_string(),
        }
    }
}
