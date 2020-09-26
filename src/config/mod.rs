use crate::definitions::{TOOL_DIR, TOOL_DEFAULT_CONFIG};
use std::io::{self, Error, ErrorKind};
use yaml_rust::YamlLoader;

#[derive(Debug)]
pub struct BldConfig {
    pub host: Option<String>,
    pub port: Option<i64>,
}

impl BldConfig {
    pub fn load() -> io::Result<Self> {
        let mut path = std::env::current_dir()?;
        path.push(TOOL_DIR);
        path.push(format!("{}.yaml", TOOL_DEFAULT_CONFIG));

        let content = std::fs::read_to_string(&path)?;

        match YamlLoader::load_from_str(&content) {
            Ok(yaml) => {
                let yaml = yaml[0].clone();
                let host = match yaml["host"].as_str() {
                    Some(host) => Some(host.to_string()),
                    None => None,
                };
                let port = match yaml["port"].as_i64() {
                    Some(port) => Some(port),
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
