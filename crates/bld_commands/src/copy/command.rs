use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::HttpClient};
use bld_utils::sync::IntoArc;
use clap::Args;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Copy a source pipeline to a target location")]
pub struct CopyCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    pub verbose: bool,

    #[arg(short = 'p', long = "pipeline", help = "The pipeline to copy")]
    pub pipeline: String,

    #[arg(short = 't', long = "target", help = "The target path")]
    pub target: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to execute the copy operation"
    )]
    pub server: Option<String>,
}

impl CopyCommand {
    fn local_copy(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.copy(&self.pipeline, &self.target)
    }

    fn remote_copy(&self, server: &str) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load()?.into_arc();
            let client = HttpClient::new(config, server);
            client.copy(&self.pipeline, &self.target).await
        })
    }
}

impl BldCommand for CopyCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        match self.server.as_ref() {
            Some(srv) => self.remote_copy(srv),
            None => self.local_copy(),
        }
    }
}
