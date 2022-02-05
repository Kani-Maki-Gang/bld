use crate::cli::BldCommand;
use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request;
use actix_web::rt::System;
use clap::{App, Arg, ArgMatches, SubCommand};
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

    fn interface(&self) -> App<'static, 'static> {
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .takes_value(true)
            .help("The name of the server from which to fetch pipeline information");
        SubCommand::with_name(LIST)
            .about("Lists information of pipelines in a bld server")
            .version(VERSION)
            .args(&vec![server])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let config = BldConfig::load()?;
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        debug!("running {} subcommand with --server: {}", LIST, srv.name);
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let url = format!("http://{}:{}/list", srv.host, srv.port);
        let headers = request::headers(name, auth)?;
        debug!("sending http request to {}", url);
        System::new().block_on(async move {
            request::get(url, headers).await.map(|r| {
                println!("{}", r);
                ()
            })
        })
    }
}
