use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::request::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Stops a running pipeline on a server")]
pub struct StopCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'i',
        long = "id",
        required = true,
        help = "The id of a pipeline running on a server"
    )]
    pipeline_id: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server that the pipeline is running"
    )]
    server: String,
}

impl BldCommand for StopCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let client = HttpClient::new(config, &self.server);
        System::new().block_on(async move { client.stop(&self.pipeline_id).await })
    }
}
