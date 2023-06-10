use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{request::Request, requests::HistQueryParams, responses::HistoryEntry};
use clap::Args;
use tabled::{Style, Table};
use tracing::debug;

#[derive(Args)]
#[command(about = "Fetches execution history of pipelines on a bld server")]
pub struct HistCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to fetch history from"
    )]
    server: String,

    #[arg(
        short = 'x',
        long = "state",
        default_value = "running",
        help = "Filter the history with state. Possible values are all, initial, queued, running, finished"
    )]
    state: String,

    #[arg(
        short = 'p',
        long = "pipeline",
        help = "Filter the history with state. Possible values are all, initial, queued, running, finished"
    )]
    pipeline: Option<String>,

    #[arg(
        short = 'l',
        long = "limit",
        default_value = "100",
        help = "Limit the results"
    )]
    limit: i64,
}

impl BldCommand for HistCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server(&self.server)?;
        let url = format!("{}/hist?", server.base_url_http());
        let params = HistQueryParams {
            state: if self.state != "all" {
                Some(self.state.to_string())
            } else {
                None
            },
            name: self.pipeline,
            limit: self.limit,
        };
        debug!(
            "running hist subcommand with --server: {} --limit {}",
            server.name, params.limit,
        );

        let request = Request::get(&url).query(&params)?.auth(server);

        debug!("sending http request to {}", url);

        System::new().block_on(async move {
            let history: Vec<HistoryEntry> = request.send().await?;
            let table = Table::new(history).with(Style::modern()).to_string();
            println!("{table}");
            Ok(())
        })
    }
}
