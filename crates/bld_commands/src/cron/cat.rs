use std::fmt::Write;

use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_models::dtos::JobFiltersParams;
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Print detailed information for a registered cron job in a server")]
pub struct CronCatCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to fetch the cron job from"
    )]
    server: String,

    #[arg(short = 'c', long = "cron-id", help = "The id of the target cron job")]
    id: String,
}

impl BldCommand for CronCatCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            let filters = JobFiltersParams::new(Some(self.id), None, None, None, None);
            let response = client.cron_list(&filters).await?;

            if !response.is_empty() {
                let entry = &response[0];
                let mut message = String::new();

                writeln!(message, "{:<13}: {}", "id", entry.id)?;
                writeln!(message, "{:<13}: {}", "schedule", entry.schedule)?;
                writeln!(message, "{:<13}: {}", "pipeline", entry.pipeline)?;
                writeln!(message, "{:<13}: {}", "is_default", entry.is_default)?;
                writeln!(message, "{:<13}: {:?}", "inputs", entry.inputs)?;
                writeln!(message, "{:<13}: {:?}", "environment", entry.env)?;
                writeln!(message)?;
                write!(
                    message,
                    "bld cron update -s {} -c {} -S '{}'",
                    self.server, entry.id, entry.schedule
                )?;

                if let Some(inputs) = &entry.inputs {
                    for (k, v) in inputs {
                        write!(message, " -i {k}='{v}'")?;
                    }
                }

                if let Some(environment) = &entry.env {
                    for (k, v) in environment {
                        write!(message, " -e {k}='{v}'")?;
                    }
                }

                print!("{message}");
            }

            Ok(())
        })
    }
}
