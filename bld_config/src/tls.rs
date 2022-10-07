use anyhow::{anyhow, Result};
use yaml_rust::Yaml;
use tracing::debug;

#[derive(Debug)]
pub struct BldTlsConfig {
    pub cert_chain: String,
    pub private_key: String,
}

impl BldTlsConfig {
    pub fn load(yaml: &Yaml) -> Result<Option<Self>> {
        if yaml.is_badvalue() {
            return Ok(None);
        }
        let cert_chain = yaml["cert-chain"]
            .as_str()
            .ok_or_else(|| anyhow!("certificate chain file not provided"))?
            .to_string();
        let private_key = yaml["private-key"]
            .as_str()
            .ok_or_else(|| anyhow!("private key file not provided"))?
            .to_string();
        Ok(Some(Self {
            cert_chain,
            private_key
        }))
    }
}
