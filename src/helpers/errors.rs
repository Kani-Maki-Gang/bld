use crate::types::{BldError, Result};

pub fn server_not_in_config() -> Result<()> {
    let message = String::from("server not found in config");
    Err(BldError::Other(message))
}

pub fn no_server_in_config() -> Result<()> {
    let message = String::from("no server found in config");
    Err(BldError::Other(message))
}

pub fn auth_for_server_invalid() -> Result<()> {
    let message = String::from("could not parse auth settings for server");
    Err(BldError::Other(message))
}
