use crate::definitions;
use std::io;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalConfig {
    pub enable_server: bool,
    pub host: String,
    pub port: i64,
    pub logs: String,
    pub db: String,
    pub docker_host: String,
    pub docker_port: i64,
    pub docker_use_tls: bool,
}

impl BldLocalConfig {
    pub fn default() -> Self {
        Self {
            enable_server: definitions::LOCAL_ENABLE_SERVER,
            host: definitions::LOCAL_SERVER_HOST.to_string(),
            port: definitions::LOCAL_SERVER_PORT,
            logs: definitions::LOCAL_LOGS.to_string(),
            db: definitions::LOCAL_DB.to_string(),
            docker_host: definitions::LOCAL_DOCKER_HOST.to_string(),
            docker_port: definitions::LOCAL_DOCKER_INSECURE_PORT,
            docker_use_tls: definitions::LOCAL_DOCKER_USE_TLS,
        }
    }

    pub fn load(yaml: &Yaml) -> io::Result<Self> {
        let local_yaml = &yaml["local"];

        let enable_server = match local_yaml["enable-server"].as_bool() {
            Some(enable_server) => enable_server,
            None => definitions::LOCAL_ENABLE_SERVER,
        };

        let host = match local_yaml["host"].as_str() {
            Some(host) => host.to_string(),
            None => definitions::LOCAL_SERVER_HOST.to_string(),
        };

        let port = match local_yaml["port"].as_i64() {
            Some(port) => port,
            None => definitions::LOCAL_SERVER_PORT,
        };

        let logs = match local_yaml["logs"].as_str() {
            Some(logs) => logs.to_string(),
            None => definitions::LOCAL_LOGS.to_string(),
        };

        let db = match local_yaml["db"].as_str() {
            Some(db) => db.to_string(),
            None => definitions::LOCAL_DB.to_string(),
        };

        let docker_host = match local_yaml["docker-host"].as_str() {
            Some(host) => host.to_string(),
            None => definitions::LOCAL_DOCKER_HOST.to_string(),
        };

        let docker_use_tls = match local_yaml["docker-use-tls"].as_bool() {
            Some(tls) => tls,
            None => definitions::LOCAL_DOCKER_USE_TLS,
        };

        let docker_port = match local_yaml["docker-port"].as_i64() {
            Some(host) => host,
            None => match docker_use_tls {
                true => definitions::LOCAL_DOCKER_SECURE_PORT,
                false => definitions::LOCAL_DOCKER_INSECURE_PORT,
            },
        };

        Ok(Self {
            enable_server,
            host,
            port,
            logs,
            db,
            docker_host,
            docker_port,
            docker_use_tls,
        })
    }
}
