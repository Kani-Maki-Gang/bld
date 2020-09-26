use crate::definitions::{TOOL_DIR, TOOL_DEFAULT_CONFIG};
use std::io::{self, Error, ErrorKind};
use yaml_rust::YamlLoader;

pub struct BldConfig {
    pub host: Option<String>,
    pub port: Option<String>,
}

impl BldConfig {
    pub fn load() -> io::Result<Self> {
        let mut path = std::env::current_dir()?;
        path.push(TOOL_DIR);
        path.push(TOOL_DEFAULT_CONFIG);

        let content = std::fs::read_to_string(&path)?;

        match YamlLoader::load_from_str(&content) {
            Ok(yaml) => {
                let host = match yaml[0]["host"].as_str() {
                    Some(host) => Some(host.to_string()),
                    None => None,
                };
                let port = match yaml[0]["port"].as_str() {
                    Some(port) => Some(port.to_string()),
                    None => None,
                };
                Ok(Self {
                    host,
                    port,
                })
            },
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }
}
