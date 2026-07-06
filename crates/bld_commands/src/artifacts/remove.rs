use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Removes an artifact from a server")]
pub struct ArtifactsRemoveCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(required = true, help = "The id of the artifact to remove")]
    id: i32,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to remove the artifact from"
    )]
    server: String,
}

impl BldCommand for ArtifactsRemoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            client.artifacts_remove(self.id).await
        })
    }
}
