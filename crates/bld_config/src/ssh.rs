use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SshUserAuth {
    #[serde(rename = "keys")]
    Keys {
        public_key: Option<String>,
        private_key: String,
    },
    #[serde(rename = "password")]
    Password { password: String },
    #[serde(rename = "agent")]
    Agent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    #[serde(default = "SshConfig::default_port")]
    pub port: String,
    pub user: String,
    pub userauth: SshUserAuth,
}

impl SshConfig {
    pub fn default_port() -> String {
        "22".to_string()
    }
}
