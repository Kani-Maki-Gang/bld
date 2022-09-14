use crate::definitions;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalSupervisorConfig {
    pub host: String,
    pub port: i64,
    pub workers: i64,
}

impl BldLocalSupervisorConfig {
    pub fn load(yaml: &Yaml) -> anyhow::Result<Self> {
        let host = yaml["host"]
            .as_str()
            .unwrap_or(definitions::LOCAL_SUPERVISOR_HOST)
            .to_string();
        let port = yaml["port"]
            .as_i64()
            .unwrap_or(definitions::LOCAL_SUPERVISOR_PORT);
        let workers = yaml["workers"]
            .as_i64()
            .unwrap_or(definitions::LOCAL_SUPERVISOR_WORKERS);
        Ok(Self {
            host,
            port,
            workers
        })
    }
}

impl Default for BldLocalSupervisorConfig {
    fn default() -> Self {
        Self {
            host: definitions::LOCAL_SUPERVISOR_HOST.to_string(),
            port: definitions::LOCAL_SUPERVISOR_PORT,
            workers: definitions::LOCAL_SUPERVISOR_WORKERS,
        }
    }
}
