use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::HttpClient};
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
    fn local_remove(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.remove(&self.pipeline)
    }

    fn remote_remove(&self, server: &str) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let client = HttpClient::new(config, server)?;

        debug!(
            "running remove subcommand with --server: {:?} and --pipeline: {}",
            self.server, self.pipeline
        );

        System::new().block_on(async move { client.remove(&self.pipeline).await })
    }
}

impl BldCommand for RemoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        match &self.server {
            Some(srv) => self.remote_remove(srv),
            None => self.local_remove(),
        }
    }
}
