use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BldTlsConfig {
    pub cert_chain: String,
    pub private_key: String,
}
