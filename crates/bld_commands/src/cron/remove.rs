use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::request::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Removes a registered cron job from a server")]
pub struct CronRemoveCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(short = 'i', long = "id", help = "The id of the cron job to remove")]
    cron_job_id: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to remove the cron job from"
    )]
    server: String,
}

impl BldCommand for CronRemoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let client = HttpClient::new(config, &self.server);
        System::new().block_on(async move { client.cron_remove(&self.cron_job_id).await })
    }
}
