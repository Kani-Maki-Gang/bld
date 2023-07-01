use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::request::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use tabled::{Style, Table};

#[derive(Args)]
#[command(about = "Lists all registered cron jobs in a server")]
pub struct CronListCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to list the cron jobs from"
    )]
    server: String,
}

impl BldCommand for CronListCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let client = HttpClient::new(config, &self.server);
        let response = System::new().block_on(async move { client.cron_list().await })?;

        if !response.is_empty() {
            let table = Table::new(response).with(Style::modern()).to_string();
            println!("{table}");
        }

        Ok(())
    }
}
