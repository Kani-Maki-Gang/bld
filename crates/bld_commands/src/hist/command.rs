use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_models::dtos::HistoryEntry;
use bld_utils::sync::IntoArc;
use clap::Args;
use tabled::{settings::Style, Table, Tabled};
use tracing::debug;

#[derive(Tabled)]
struct HistoryEntryRow {
    pub name: String,
    pub id: String,
    pub user: String,
    pub state: String,
    #[tabled(display_with = "HistoryEntryRow::display_option")]
    pub start_date_time: Option<String>,
    #[tabled(display_with = "HistoryEntryRow::display_option")]
    pub end_date_time: Option<String>,
}

impl HistoryEntryRow {
    pub fn display_option(value: &Option<String>) -> String {
        value.as_deref().unwrap_or("").to_string()
    }
}

impl From<HistoryEntry> for HistoryEntryRow {
    fn from(value: HistoryEntry) -> Self {
        Self {
            name: value.name,
            id: value.id,
            user: value.user,
            state: value.state,
            start_date_time: value.start_date_time,
            end_date_time: value.end_date_time,
        }
    }
}

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
    limit: u64,
}

impl BldCommand for HistCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();

            let state = if self.state != "all" {
                Some(self.state.to_string())
            } else {
                None
            };

            debug!(
                "running hist subcommand with --server: {:?} --limit {}",
                self.server, self.limit,
            );

            let history: Vec<HistoryEntryRow> = HttpClient::new(config, &self.server)?
                .hist(state, self.pipeline, self.limit)
                .await?
                .into_iter()
                .map(From::from)
                .collect();

            if !history.is_empty() {
                let table = Table::new(history).with(Style::modern()).to_string();
                println!("{table}");
            }

            Ok(())
        })
    }
}
