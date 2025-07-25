use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_models::dtos::JobFiltersParams;
use bld_utils::sync::IntoArc;
use clap::Args;
use tabled::{Table, Tabled, settings::Style};

#[derive(Tabled)]
struct JobInfoRow<'a> {
    pub id: &'a str,
    pub schedule: &'a str,
    pub pipeline: &'a str,
    pub is_default: bool,
    pub date_created: &'a str,
    pub date_updated: &'a str,
}

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

    #[arg(
        short = 'p',
        long = "pipeline",
        help = "The pipeline name for the target cron jobs"
    )]
    pipeline: Option<String>,

    #[arg(
        short = 'S',
        long = "schedule",
        help = "The schedule for the target cron jobs"
    )]
    schedule: Option<String>,

    #[arg(
        short = 'd',
        long = "default",
        help = "Fetch only the default cron jobs"
    )]
    is_default: Option<bool>,

    #[arg(short = 'l', long = "limit", help = "Limit the results")]
    limit: Option<usize>,
}

impl BldCommand for CronListCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            let filters = JobFiltersParams::new(
                None,
                self.pipeline,
                self.schedule,
                self.is_default,
                self.limit.map(|x| x as u64),
            );
            let response = client.cron_list(&filters).await?;

            if !response.is_empty() {
                let data: Vec<JobInfoRow> = response
                    .iter()
                    .map(|j| JobInfoRow {
                        id: &j.id,
                        schedule: &j.schedule,
                        pipeline: &j.pipeline,
                        is_default: j.is_default,
                        date_created: &j.date_created,
                        date_updated: j.date_updated.as_deref().unwrap_or(""),
                    })
                    .collect();
                let table = Table::new(data).with(Style::modern()).to_string();
                println!("{table}");
            }

            Ok(())
        })
    }
}
