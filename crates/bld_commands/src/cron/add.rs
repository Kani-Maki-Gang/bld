use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{request::HttpClient, requests::AddJobRequest};
use bld_utils::{sync::IntoArc, variables::parse_variables};
use clap::Args;

#[derive(Args)]
#[command(about = "Adds a cron job to a server")]
pub struct CronAddCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to upsert the cron job to"
    )]
    server: String,

    #[arg(
        short = 'p',
        long = "pipeline",
        help = "The name of the target pipeline"
    )]
    pipeline: String,

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

impl BldCommand for CronAddCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let client = HttpClient::new(config, &self.server);
        let variables = Some(parse_variables(&self.variables));
        let environment = Some(parse_variables(&self.environment));
        let request =
            AddJobRequest::new(self.schedule, self.pipeline, variables, environment, false);
        System::new().block_on(async move { client.cron_add(&request).await })
    }
}
