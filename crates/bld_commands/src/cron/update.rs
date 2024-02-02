use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_dtos::UpdateJobRequest;
use bld_http::HttpClient;
use bld_utils::{sync::IntoArc, variables::parse_variables};
use clap::Args;

#[derive(Args)]
#[command(about = "Updates a cron job to a server")]
pub struct CronUpdateCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to upsert the cron job to"
    )]
    server: String,

    #[arg(short = 'i', long = "id", help = "The id of the target cron job")]
    id: String,

    #[arg(
        short = 'S',
        long = "schedule",
        help = "The new schedule for the cron job"
    )]
    schedule: String,

    #[arg(
        short = 'v',
        long = "variable",
        help = "Define value for a variable. Can be used multiple times"
    )]
    variables: Vec<String>,

    #[arg(
        short = 'e',
        long = "environment",
        help = "Define value for an environment variable. Can be used multiple times"
    )]
    environment: Vec<String>,
}

impl BldCommand for CronUpdateCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            let variables = Some(parse_variables(&self.variables));
            let environment = Some(parse_variables(&self.environment));
            let update_job = UpdateJobRequest::new(self.id, self.schedule, variables, environment);
            client.cron_update(&update_job).await
        })
    }
}
