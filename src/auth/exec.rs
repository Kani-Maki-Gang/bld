use crate::auth::Login;
use crate::config::BldConfig;
use crate::helpers::errors::{no_server_in_config, server_not_in_config};
use crate::types::Result;
use clap::ArgMatches;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
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
    srv.login()?;
    Ok(())
}
