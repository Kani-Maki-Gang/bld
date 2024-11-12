use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_models::dtos::AddJobRequest;
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
        short = 'i',
        long = "input",
        help = "Define value for an input variable. Can be used multiple times"
    )]
    inputs: Vec<String>,

    #[arg(
        short = 'e',
        long = "environment",
        help = "Define value for an environment variable. Can be used multiple times"
    )]
    env: Vec<String>,
}

impl BldCommand for CronAddCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            let inputs = Some(parse_variables(&self.inputs));
            let env = Some(parse_variables(&self.env));
            let request =
                AddJobRequest::new(self.schedule, self.pipeline, inputs, env, false);
            client.cron_add(&request).await
        })
    }
}
