use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::{request::Request, sync::IntoArc};
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Removes a pipeline from a bld server")]
pub struct RemoveCommand {
    #[arg(short = 's', long = "server", help = "The name of the bld server")]
    server: Option<String>,

    #[arg(short = 'p', long = "pipeline", help = "The name of the pipeline")]
    pipeline: String,
}

impl RemoveCommand {
    fn local_exec(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.remove(&self.pipeline)
    }

    fn server_exec(&self, server: &str) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server(server)?;

        debug!(
            "running remove subcommand with --server: {} and --pipeline: {}",
            server.name, self.pipeline
        );

        let server_auth = config.same_auth_as(server)?;
        let url = format!("{}/remove", server.base_url_http());
        let request = Request::post(&url).auth(server_auth);

        System::new().block_on(async move {
            debug!("sending request to {}", url);
            request.send_json(&self.pipeline).await.map(|r: String| {
                println!("{r}");
            })
        })
    }
}

impl BldCommand for RemoveCommand {
    fn exec(self) -> Result<()> {
        match &self.server {
            Some(srv) => self.server_exec(srv),
            None => self.local_exec(),
        }
    }
}
