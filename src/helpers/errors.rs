use crate::types::{BldError, Result};

pub fn err_variable_in_yaml() -> BldError {
    let message = String::from("error in variable section");
    BldError::Other(message)
}

pub fn err_server_not_in_config() -> BldError {
    let message = String::from("server not found in config");
    BldError::Other(message)
}

pub fn err_no_server_in_config() -> BldError {
    let message = String::from("no server found in config");
    BldError::Other(message)
}

pub fn auth_for_server_invalid() -> Result<()> {
    let message = String::from("could not parse auth settings for server");
    Err(BldError::Other(message))
}
