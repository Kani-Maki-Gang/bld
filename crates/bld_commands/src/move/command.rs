use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::HttpClient};
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
    fn local_move(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.mv(&self.pipeline, &self.target)
    }

    fn remote_move(&self, server: &str) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load()?.into_arc();
            HttpClient::new(config, server)?
                .mv(&self.pipeline, &self.target)
                .await
        })
    }
}

impl BldCommand for MoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        match self.server.as_ref() {
            Some(srv) => self.remote_move(srv),
            None => self.local_move(),
        }
    }
}
