use crate::types::Result;
use crate::config::{BldConfig, definitions::TOOL_DEFAULT_PIPELINE};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_get, headers};
use clap::ArgMatches;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let pip = matches
        .value_of("pipeline")
        .or_else(|| Some(TOOL_DEFAULT_PIPELINE))
        .unwrap()
        .to_string();
    let srv = config.remote.server_or_first(matches.value_of("server"))?;
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        }
        None => (&srv.name, &srv.auth),
    };
    let url = format!("http://{}:{}/inspect/{}", srv.host, srv.port, pip);
    let headers = headers(name, auth)?;
    let sys = String::from("bld-inspect");
    exec_get(sys, url, headers);
    Ok(())
}
