use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::definitions::TOOL_DEFAULT_PIPELINE_FILE;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_http::HttpClient;
use bld_runner::{Load, Yaml};
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Checks a pipeline file for errors")]
pub struct CheckCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(short = 'p', long = "pipeline", default_value = TOOL_DEFAULT_PIPELINE_FILE, help = "Path to pipeline script")]
    pipeline: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to check the pipeline from"
    )]
    server: Option<String>,
}

impl CheckCommand {
    async fn local_check(&self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let content = fs.read(&self.pipeline).await?;
        let pipeline = Yaml::load_with_verbose_errors(&content)?;
        pipeline.validate_with_verbose_errors(config, fs).await
    }

    async fn remote_check(&self, server: &str) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        HttpClient::new(config, server)?.check(&self.pipeline).await
    }
}

impl BldCommand for CheckCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            match &self.server {
                Some(server) => self.remote_check(server).await,
                None => self.local_check().await,
            }
        })
    }
}
