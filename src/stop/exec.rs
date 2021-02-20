use crate::config::BldConfig;
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_post, headers};
use crate::types::Result;
use clap::ArgMatches;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let id = matches.value_of("id").unwrap().to_string();
    let srv = config.remote.server_or_first(matches.value_of("server"))?;
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    let sys = String::from("bld-stop");
    let url = format!("http://{}:{}/stop", srv.host, srv.port);
    let headers = headers(name, auth)?;
    exec_post(sys, url, headers, id);
    Ok(())
}
