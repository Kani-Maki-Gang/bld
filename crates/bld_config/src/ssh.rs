use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    #[serde(
        default = "SshConfig::default_port",
        deserialize_with = "SshConfig::deserialize_port"
    )]
    pub port: String,
    pub user: String,
    pub userauth: SshUserAuth,
}

impl SshConfig {
    pub fn default_port() -> String {
        "22".to_string()
    }

    fn deserialize_port<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Port {
            Number(u16),
            Text(String),
        }

        match Port::deserialize(deserializer) {
            Ok(Port::Number(port)) => Ok(port.to_string()),
            Ok(Port::Text(port)) => Ok(port),
            Err(_) => Err(serde::de::Error::custom(
                "invalid port, expected a number between 0 and 65535 or an expression",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SshConfig;

    fn parse(port: &str) -> Result<SshConfig, serde_yaml_ng::Error> {
        let yaml = format!("host: localhost\n{port}user: bld\nuserauth:\n  type: agent\n");
        serde_yaml_ng::from_str(&yaml)
    }

    #[test]
    pub fn ssh_config_unquoted_port_deserialize_success() {
        assert_eq!(parse("port: 2222\n").unwrap().port, "2222");
    }

    #[test]
    pub fn ssh_config_quoted_port_deserialize_success() {
        assert_eq!(parse("port: \"2222\"\n").unwrap().port, "2222");
    }

    #[test]
    pub fn ssh_config_expression_port_deserialize_success() {
        let config = parse("port: ${{ inputs.port }}\n").unwrap();
        assert_eq!(config.port, "${{ inputs.port }}");
    }

    #[test]
    pub fn ssh_config_missing_port_deserialize_success() {
        assert_eq!(parse("").unwrap().port, "22");
    }

    #[test]
    pub fn ssh_config_out_of_range_port_deserialize_failure() {
        assert!(parse("port: 70000\n").is_err());
    }
}
