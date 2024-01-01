use std::collections::HashMap;

#[derive(Debug, )]
pub enum DockerUrl {
    SingleUrl(String),
    MultipleUrls(HashMap<String, String>),
}
