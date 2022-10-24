use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing::debug;

static SERVER: &str = "server";
static HOST: &str = "host";
static PORT: &str = "port";

pub struct ServerCommand;

impl BldCommand for ServerCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        SERVER
    }

    fn interface(&self) -> Command {
        let host = Arg::new(HOST)
            .long("host")
            .short('H')
            .help("The server's host address")
            .action(ArgAction::Set);

        let port = Arg::new(PORT)
            .long("port")
            .short('P')
            .help("The server's port")
            .action(ArgAction::Set);

        Command::new(SERVER)
            .about("Start bld in server mode, listening to incoming build requests")
            .version(VERSION)
            .args(&[host, port])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;

        let host = matches
            .get_one::<String>("host")
            .unwrap_or(&config.local.server.host)
            .to_string();

        let port = matches
            .get_one::<String>("port")
            .map(|port| port.parse::<i64>().unwrap_or(config.local.server.port))
            .unwrap_or(config.local.server.port);

        debug!("running {SERVER} subcommand with --host: {host} --port: {port}",);

        System::new().block_on(async move { bld_server::start(config, host, port).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_server_host_arg_accepts_value() {
        let host = "mock_host";
        let command = ServerCommand::boxed().interface();
        let matches = command.get_matches_from(&["server", "-H", host]);

        assert_eq!(matches.get_one::<String>(HOST), Some(&host.to_string()))
    }

    #[test]
    fn cli_server_port_arg_accepts_value() {
        let port = "mock_port";
        let command = ServerCommand::boxed().interface();
        let matches = command.get_matches_from(&["server", "-P", port]);

        assert_eq!(matches.get_one::<String>(PORT), Some(&port.to_string()))
    }
}
