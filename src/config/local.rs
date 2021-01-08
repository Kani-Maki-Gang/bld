use crate::config::definitions;
use crate::config::AuthValidation;
use crate::types::Result;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalConfig {
    pub server_mode: bool,
    pub host: String,
    pub port: i64,
    pub logs: String,
    pub db: String,
    pub auth: AuthValidation,
    pub docker_url: String,
}

impl BldLocalConfig {
    pub fn default() -> Self {
        Self {
            server_mode: definitions::LOCAL_SERVER_MODE,
            host: definitions::LOCAL_SERVER_HOST.to_string(),
            port: definitions::LOCAL_SERVER_PORT,
            logs: definitions::LOCAL_LOGS.to_string(),
            db: definitions::LOCAL_DB.to_string(),
            auth: AuthValidation::None,
            docker_url: definitions::LOCAL_DOCKER_URL.to_string(),
        }
    }

    pub fn load(yaml: &Yaml) -> Result<Self> {
        let local_yaml = &yaml["local"];
        let server_mode = local_yaml["server-mode"]
            .as_bool()
            .or(Some(definitions::LOCAL_SERVER_MODE))
            .unwrap();
        let host = local_yaml["host"]
            .as_str()
            .or(Some(definitions::LOCAL_SERVER_HOST))
            .unwrap()
            .to_string();
        let port = local_yaml["port"]
            .as_i64()
            .or(Some(definitions::LOCAL_SERVER_PORT))
            .unwrap();
        let logs = local_yaml["logs"]
            .as_str()
            .or(Some(definitions::LOCAL_LOGS))
            .unwrap()
            .to_string();
        let db = local_yaml["db"]
            .as_str()
            .or(Some(definitions::LOCAL_DB))
            .unwrap()
            .to_string();
        let docker_url = local_yaml["docker-url"]
            .as_str()
            .or(Some(definitions::LOCAL_DOCKER_URL))
            .unwrap()
            .to_string();
        let auth = BldLocalConfig::auth_load(local_yaml)?;
        Ok(Self {
            server_mode,
            host,
            port,
            logs,
            db,
            auth,
            docker_url,
        })
    }

    fn auth_load(yaml: &Yaml) -> Result<AuthValidation> {
        let auth_validation = match yaml["auth"]["method"].as_str() {
            Some("ldap") => AuthValidation::Ldap,
            Some("oauth2") => AuthValidation::OAuth2(
                yaml["auth"]["validation-url"]
                    .as_str()
                    .ok_or("no validation url found for auth in config")?
                    .to_string(),
            ),
            _ => AuthValidation::None,
        };
        Ok(auth_validation)
    }
}
