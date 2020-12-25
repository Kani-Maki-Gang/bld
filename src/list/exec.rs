use crate::config::BldConfig;
use crate::helpers::errors::{no_server_in_config, server_not_in_config};
use crate::helpers::request::exec_get;
use crate::types::Result;
use clap::ArgMatches;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let (host, port) = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => (srv.host.to_string(), srv.port),
            None => return server_not_in_config(),
        },
        None => match servers.iter().next() {
            Some(srv) => (srv.host.to_string(), srv.port),
            None => return no_server_in_config(),
        },
    };
    let sys = String::from("bld-ls");
    let url = format!("http://{}:{}/list", host, port);
    exec_get(sys, url);
    Ok(())
}
