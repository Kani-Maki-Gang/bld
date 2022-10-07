use crate::definitions;
use crate::BldTlsConfig;
use anyhow::Result;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldLocalSupervisorConfig {
    pub host: String,
    pub port: i64,
    pub tls: Option<BldTlsConfig>,
    pub workers: i64,
}

impl BldLocalSupervisorConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let host = yaml["host"]
            .as_str()
            .unwrap_or(definitions::LOCAL_SUPERVISOR_HOST)
            .to_string();
        let port = yaml["port"]
            .as_i64()
            .unwrap_or(definitions::LOCAL_SUPERVISOR_PORT);
        let tls = BldTlsConfig::load(&yaml["tls"])?;
        let workers = yaml["workers"]
            .as_i64()
            .unwrap_or(definitions::LOCAL_SUPERVISOR_WORKERS);
        Ok(Self {
            host,
            port,
            tls,
            workers,
        })
    }
}

impl Default for BldLocalSupervisorConfig {
    fn default() -> Self {
        Self {
            host: definitions::LOCAL_SUPERVISOR_HOST.to_string(),
            port: definitions::LOCAL_SUPERVISOR_PORT,
            tls: None,
            workers: definitions::LOCAL_SUPERVISOR_WORKERS,
        }
    }
}
