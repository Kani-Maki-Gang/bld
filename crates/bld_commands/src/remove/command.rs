use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Removes a pipeline")]
pub struct RemoveCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to remove from"
    )]
    server: Option<String>,

    #[arg(short = 'p', long = "pipeline", help = "The name of the pipeline")]
    pipeline: String,
}

impl RemoveCommand {
    async fn local_remove(&self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let fs = FileSystem::local(config);
        fs.remove(&self.pipeline).await
    }

    async fn remote_remove(&self, server: &str) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let client = HttpClient::new(config, server)?;

        debug!(
            "running remove subcommand with --server: {:?} and --pipeline: {}",
            self.server, self.pipeline
        );

        client.remove(&self.pipeline).await
    }
}

impl BldCommand for RemoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            match &self.server {
                Some(srv) => self.remote_remove(srv).await,
                None => self.local_remove().await,
            }
        })
    }
}
