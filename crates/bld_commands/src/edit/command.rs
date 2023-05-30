use actix::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_server::{requests::PushInfo, responses::PullResponse};
use bld_utils::{request::Request, sync::IntoArc};
use clap::Args;
use tracing::debug;
use uuid::Uuid;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Edit a pipeline file")]
pub struct EditCommand {
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
    fn local_edit(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.edit(&self.pipeline)
    }

    fn remote_edit(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load()?.into_arc();
            let proxy = PipelineFileSystemProxy::local(config.clone());
            let server_name = self.server.ok_or_else(|| anyhow!("server not found"))?;
            let server = config.server(&server_name)?;
            let server_auth = config.same_auth_as(server)?;
            let pull_url = format!("{}/pull", server.base_url_http());

            println!("Pulling pipline {}", self.pipeline);

            debug!("sending request to {pull_url}");

            let data: PullResponse = Request::post(&pull_url)
                .auth(server_auth)
                .send_json(&self.pipeline)
                .await?;

            let tmp_name = format!("{}{}", Uuid::new_v4(), data.name);

            println!("Editing temporary local pipeline {}", tmp_name);

            debug!("creating temporary pipeline file: {tmp_name}");
            proxy.create_tmp(&tmp_name, &data.content, true)?;

            debug!("starting editor for temporary pipeline file: {tmp_name}");
            proxy.edit_tmp(&tmp_name)?;

            debug!("reading content of temporary pipeline file: {tmp_name}");
            let tmp_content = proxy.read_tmp(&tmp_name)?;

            let push_url = format!("{}/push", server.base_url_http());
            let push_info = PushInfo::new(&self.pipeline, &tmp_content);

            println!("Pushing updated content for {}", self.pipeline);

            debug!("sending request to {push_url}");

            let _: String = Request::post(&push_url)
                .auth(server_auth)
                .send_json(&push_info)
                .await?;

            debug!("deleting temporary pipeline file: {tmp_name}");
            proxy.remove_tmp(&tmp_name)?;

            Ok(())
        })
    }
}

impl BldCommand for EditCommand {
    fn exec(self) -> Result<()> {
        debug!(
            "running edit subcommand with --server: {:?} and --pipeline: {}",
            self.server, self.pipeline
        );

        match &self.server {
            Some(_) => self.remote_edit(),
            None => self.local_edit(),
        }
    }
}
