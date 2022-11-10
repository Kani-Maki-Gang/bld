use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::{definitions::VERSION, BldConfig};
use bld_server::responses::HistoryEntry;
use bld_utils::request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::fmt::Write;
use tabled::{Style, Table};
use tracing::debug;

const HIST: &str = "hist";
const SERVER: &str = "server";
const STATE: &str = "state";
const STATE_VALUE_ALL: &str = "all";
const PIPELINE: &str = "pipeline";
const LIMIT: &str = "limit";

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
            .long(SERVER)
            .action(ArgAction::Set)
            .help("The name of the server from which to fetch execution history");

        let state = Arg::new(STATE)
            .short('x')
            .long(STATE)
            .action(ArgAction::Set)
            .default_value("running")
            .help("Filter the history with state. Possible values are all, initial, queued, running, finished");

        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .action(ArgAction::Set)
            .help("Filter histort with pipeline name");

        let limit = Arg::new(LIMIT)
            .short('l')
            .long(LIMIT)
            .action(ArgAction::Set)
            .default_value("100")
            .help("Limit the results");

        Command::new(HIST)
            .about("Fetches execution history of pipelines on a server")
            .version(VERSION)
            .args(&[server, state, pipeline, limit])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;

        let state = matches.get_one::<String>(STATE).unwrap();
        let pipeline = matches.get_one::<String>(PIPELINE);
        let limit = matches.get_one::<String>(LIMIT).unwrap().parse::<i64>()?;
        debug!(
            "running {} subcommand with --server: {} --limit {limit}",
            HIST, server.name
        );

        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let mut url = format!(
            "{protocol}://{}:{}/hist?",
            server.host, server.port
        );

        if state != STATE_VALUE_ALL {
            write!(url, "state={state}")?;
        }

        if let Some(pipeline) = pipeline {
            write!(url, "&name={pipeline}")?;
        }

        write!(url, "&limit={limit}")?;

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
