use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::{definitions::VERSION, BldConfig};
use bld_server::responses::HistoryEntry;
use bld_utils::request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tabled::{Style, Table};
use tracing::debug;

static HIST: &str = "hist";
static SERVER: &str = "server";

pub struct HistCommand;

impl BldCommand for HistCommand {
    fn boxed() -> Box<Self> {
        Box::new(HistCommand)
    }

    fn id(&self) -> &'static str {
        HIST
    }

    fn interface(&self) -> Command {
        let server = Arg::new(SERVER)
            .short('s')
            .long("server")
            .action(ArgAction::Set)
            .help("The name of the server from which to fetch execution history");

        Command::new(HIST)
            .about("Fetches execution history of pipelines on a server")
            .version(VERSION)
            .args(&[server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_hist_server_arg_accepts_value() {
        let server_name = "mockServer";
        let command = HistCommand::boxed().interface();
        let matches = command.get_matches_from(&["hist", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>(SERVER),
            Some(&server_name.to_string())
        )
    }
}
