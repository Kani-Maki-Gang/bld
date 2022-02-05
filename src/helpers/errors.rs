use anyhow::anyhow;

pub fn err_variable_in_yaml() -> anyhow::Error {
    anyhow!("error in variable section")
}

pub fn err_server_not_in_config() -> anyhow::Error {
    anyhow!("server not found in config")
}

pub fn err_no_server_in_config() -> anyhow::Error {
    anyhow!("no server found in config")
}

pub fn auth_for_server_invalid() -> anyhow::Result<()> {
    Err(anyhow!("could not parse auth settings for server"))
}
