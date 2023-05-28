use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::definitions::TOOL_DEFAULT_PIPELINE_FILE;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::{Load, Yaml};
use bld_server::requests::CheckQueryParams;
use bld_utils::{request::Request, sync::IntoArc};
use clap::Args;

#[derive(Args)]
#[command(about = "Checks a pipeline file for errors")]
pub struct CheckCommand {
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
    fn exec_local(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config.clone()).into_arc();
        let content = proxy.read(&self.pipeline)?;
        let pipeline = Yaml::load_with_verbose_errors(&content)?;
        pipeline.validate_with_verbose_errors(config, proxy)
    }

    fn exec_server(&self, server: &str) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server(server)?;
        let server_auth = config.same_auth_as(server)?;

        let url = format!("{}/check", server.base_url_http());
        let request = Request::get(&url)
            .auth(server_auth)
            .query(&CheckQueryParams {
                pipeline: self.pipeline.to_owned(),
            })?;

        System::new().block_on(async move { request.send::<String>().await.map(|_| ()) })
    }
}

impl BldCommand for CheckCommand {
    fn exec(self) -> Result<()> {
        match &self.server {
            Some(server) => self.exec_server(server),
            None => self.exec_local(),
        }
    }
}
