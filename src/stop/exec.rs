use crate::config::BldConfig;
use crate::helpers::errors::{no_server_in_config, server_not_in_config};
use crate::helpers::request::{exec_post, headers};
use crate::types::Result;
use clap::ArgMatches;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let id = matches.value_of("id").unwrap().to_string();
    let srv = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => srv,
            None => return server_not_in_config(),
        },
        None => match servers.iter().next() {
            Some(srv) => srv,
            None => return no_server_in_config(),
        },
    };
    let sys = String::from("bld-stop");
    let url = format!("http://{}:{}/stop", srv.host, srv.port);
    let headers = headers(&srv.name, &srv.auth)?;
    exec_post(sys, url, headers, id);
    Ok(())
}
