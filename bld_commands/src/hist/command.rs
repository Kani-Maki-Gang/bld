use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::{definitions::VERSION, BldConfig};
use bld_server::responses::HistoryEntry;
use bld_utils::request;
use clap::{App, Arg, ArgMatches, SubCommand};
use tabled::{Style, Table};
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
        let server = config.remote.server_or_first(matches.value_of(SERVER))?;
        debug!("running {} subcommand with --server: {}", HIST, server.name);
        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/hist", server.host, server.port);
        let headers = request::headers(&server_auth.name, &server_auth.auth)?;
        debug!("sending http request to {}", url);
        System::new().block_on(async move {
            let res = request::get(url, headers).await?;
            let history: Vec<HistoryEntry> = serde_json::from_str(&res)?;
            let table = Table::new(history).with(Style::modern()).to_string();
            println!("{table}");
            Ok(())
        })
    }
}
