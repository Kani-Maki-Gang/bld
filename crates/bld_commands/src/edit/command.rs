use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;
use uuid::Uuid;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Edit a pipeline file")]
pub struct EditCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(short = 'p', long = "pipline", help = "The name of the pipeline file")]
    pipeline: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to edit the pipeline from"
    )]
    server: Option<String>,
}

impl EditCommand {
    async fn local_edit(&self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.edit(&self.pipeline).await
    }

    async fn remote_edit(&self, server: &str) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let client = HttpClient::new(config.clone(), server)?;
        let proxy = PipelineFileSystemProxy::local(config);
        println!("Pulling pipline {}", self.pipeline);

        let data = client.pull(&self.pipeline).await?;

        let tmp_name = format!("{}.yaml", Uuid::new_v4());

        println!("Editing temporary local pipeline {}", tmp_name);

        debug!("creating temporary pipeline file: {tmp_name}");
        proxy.create_tmp(&tmp_name, &data.content, true).await?;

        debug!("starting editor for temporary pipeline file: {tmp_name}");
        proxy.edit_tmp(&tmp_name).await?;

        debug!("reading content of temporary pipeline file: {tmp_name}");
        let tmp_content = proxy.read_tmp(&tmp_name).await?;

        println!("Pushing updated content for {}", self.pipeline);

        client.push(&self.pipeline, &tmp_content).await?;

        debug!("deleting temporary pipeline file: {tmp_name}");
        proxy.remove_tmp(&tmp_name).await?;

        Ok(())
    }
}

impl BldCommand for EditCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        debug!(
            "running edit subcommand with --server: {:?} and --pipeline: {}",
            self.server, self.pipeline
        );

        System::new().block_on(async move {
            match &self.server {
                Some(srv) => self.remote_edit(srv).await,
                None => self.local_edit().await,
            }
        })
    }
}
