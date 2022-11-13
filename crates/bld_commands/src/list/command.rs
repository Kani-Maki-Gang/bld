use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_utils::request::Request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing::debug;

static LIST: &str = "ls";
static SERVER: &str = "server";

pub struct ListCommand;

impl BldCommand for ListCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        LIST
    }

    fn interface(&self) -> Command {
        let server = Arg::new(SERVER)
            .short('s')
            .long("server")
            .help("The name of the server from which to fetch pipeline information")
            .action(ArgAction::Set);

        Command::new(LIST)
            .about("Lists information of pipelines in a bld server")
            .version(VERSION)
            .args(&vec![server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;

        debug!("running {} subcommand with --server: {}", LIST, server.name);

        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/list", server.host, server.port);
        let request = Request::get(&url).auth(&server_auth);

        debug!("sending {protocol} request to {}", url);

        System::new().block_on(async move { request.send().await.map(|r: String| println!("{}", r.to_string())) })
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::BldCommand;
    use crate::list::ListCommand;

    #[test]
    fn cli_list_server_arg_accepts_value() {
        let server_name = "mockServer";
        let command = ListCommand::boxed().interface();
        let matches = command.get_matches_from(&["hist", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>("server"),
            Some(&server_name.to_string())
        )
    }
}
