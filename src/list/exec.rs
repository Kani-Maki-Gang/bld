use crate::config::BldConfig;
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_get, headers};
use crate::types::Result;
use clap::ArgMatches;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let srv = config.remote.server_or_first(matches.value_of("server"))?;
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    let sys = String::from("bld-ls");
    let url = format!("http://{}:{}/list", srv.host, srv.port);
    let headers = headers(name, auth)?;
    exec_get(sys, url, headers);
    Ok(())
}
