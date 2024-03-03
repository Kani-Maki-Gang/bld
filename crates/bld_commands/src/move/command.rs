use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Move a source pipeline to a target location")]
pub struct MoveCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    pub verbose: bool,

    #[arg(short = 'p', long = "pipeline", help = "The pipeline to move")]
    pub pipeline: String,

    #[arg(short = 't', long = "target", help = "The target path")]
    pub target: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to execute the move operation"
    )]
    pub server: Option<String>,
}

impl MoveCommand {
    async fn local_move(&self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let fs = FileSystem::local(config);
        fs.mv(&self.pipeline, &self.target).await
    }

    async fn remote_move(&self, server: &str) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        HttpClient::new(config, server)?
            .mv(&self.pipeline, &self.target)
            .await
    }
}

impl BldCommand for MoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            match self.server.as_ref() {
                Some(srv) => self.remote_move(srv).await,
                None => self.local_move().await,
            }
        })
    }
}
