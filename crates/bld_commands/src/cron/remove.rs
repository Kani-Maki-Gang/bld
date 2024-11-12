use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Removes a registered cron job from a server")]
pub struct CronRemoveCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(short = 'c', long = "cron-id", help = "The id of the cron job to remove")]
    id: String,

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
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            client.cron_remove(&self.id).await
        })
    }
}
