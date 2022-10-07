use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::{definitions::VERSION, BldConfig};
use bld_utils::errors::auth_for_server_invalid;
use bld_utils::request;
use clap::{App, Arg, ArgMatches, SubCommand};
use tracing::debug;

static HIST: &str = "hist";
static SERVER: &str = "server";

pub struct HistCommand;

impl HistCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(HistCommand)
    }
}

impl BldCommand for HistCommand {
    fn id(&self) -> &'static str {
        HIST
    }

    fn interface(&self) -> App<'static> {
        let server = Arg::with_name(SERVER)
            .short('s')
            .long("server")
            .takes_value(true)
            .help("The name of the server from which to fetch execution history");
        SubCommand::with_name(HIST)
            .about("Fetches execution history of pipelines on a server")
            .version(VERSION)
            .args(&[server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        debug!("running {} subcommand with --server: {}", HIST, srv.name);
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let protocol = srv.http_protocol();
        let url = format!("{protocol}://{}:{}/hist", srv.host, srv.port);
        let headers = request::headers(name, auth)?;
        debug!("sending http request to {}", url);
        System::new().block_on(async move {
            request::get(url, headers).await.map(|r| {
                println!("{r}");
            })
        })
    }
}
