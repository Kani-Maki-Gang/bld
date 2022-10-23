use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_utils::request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing::debug;

static LIST: &str = "ls";
static SERVER: &str = "server";

pub struct ListCommand;

impl ListCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for ListCommand {
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
        let headers = request::headers(&server_auth.name, &server_auth.auth)?;

        debug!("sending {protocol} request to {}", url);

        System::new()
            .block_on(async move { request::get(url, headers).await.map(|r| println!("{r}")) })
    }
}
