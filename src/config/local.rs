use crate::config::definitions;
use std::io;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalConfig {
    pub enable_server: bool,
    pub host: String,
    pub port: i64,
    pub logs: String,
    pub db: String,
    pub docker_url: String,
}

impl BldLocalConfig {
    pub fn default() -> Self {
        Self {
            enable_server: definitions::LOCAL_ENABLE_SERVER,
            host: definitions::LOCAL_SERVER_HOST.to_string(),
            port: definitions::LOCAL_SERVER_PORT,
            logs: definitions::LOCAL_LOGS.to_string(),
            db: definitions::LOCAL_DB.to_string(),
            docker_url: definitions::LOCAL_DOCKER_URL.to_string(),
        }
    }

    pub fn load(yaml: &Yaml) -> io::Result<Self> {
        let local_yaml = &yaml["local"];
        let enable_server = local_yaml["enable-server"]
            .as_bool()
            .or(Some(definitions::LOCAL_ENABLE_SERVER))
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
        Ok(Self {
            enable_server,
            host,
            port,
            logs,
            db,
            docker_url,
        })
    }
}
