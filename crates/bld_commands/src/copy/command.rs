use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Copy a source file to a target location")]
pub struct CopyCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    pub verbose: bool,

    #[arg(short = 'f', long = "file", help = "The file to copy")]
    pub file: String,

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
    async fn local_copy(&self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let fs = FileSystem::local(config);
        fs.copy(&self.file, &self.target).await
    }

    async fn remote_copy(&self, server: &str) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let client = HttpClient::new(config, server)?;
        client.copy(&self.file, &self.target).await
    }
}

impl BldCommand for CopyCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            match self.server.as_ref() {
                Some(srv) => self.remote_copy(srv).await,
                None => self.local_copy().await,
            }
        })
    }
}
