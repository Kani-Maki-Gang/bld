use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::HttpClient};
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "List pipelines")]
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
    async fn local_list(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        let content = proxy.list().await?.join("\n");
        println!("{content}");
        Ok(())
    }

    async fn remote_list(&self, server: &str) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        HttpClient::new(config, server)?
            .list()
            .await
            .map(|r| println!("{r}"))
    }
}

impl BldCommand for ListCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            match &self.server {
                Some(srv) => self.remote_list(srv).await,
                None => self.local_list().await,
            }
        })
    }
}
