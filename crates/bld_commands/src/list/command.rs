use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::Request};
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Lists information of pipelines in a bld server")]
pub struct ListCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to list pipelines from"
    )]
    server: Option<String>,
}

impl ListCommand {
    fn local_list(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        let content = proxy.list()?.join("\n");
        println!("{content}");
        Ok(())
    }

    fn remote_list(&self, server: &str) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server(server)?;
        let url = format!("{}/list", server.base_url_http());
        let request = Request::get(&url).auth(server);

        debug!("sending request to {}", url);

        System::new().block_on(async move { request.send().await.map(|r: String| println!("{r}")) })
    }
}

impl BldCommand for ListCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        match &self.server {
            Some(srv) => self.remote_list(srv),
            None => self.local_list(),
        }
    }
}
