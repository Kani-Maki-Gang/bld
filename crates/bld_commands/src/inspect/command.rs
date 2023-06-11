use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::HttpClient};
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Inspects the contents of a pipeline on a bld server")]
pub struct InspectCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'p',
        long = "pipeline",
        required = true,
        help = "The name of the pipeline to inspect"
    )]
    pipeline: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to inspect the pipeline from"
    )]
    server: Option<String>,
}

impl InspectCommand {
    fn local_inspect(&self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        let pipeline = proxy.read(&self.pipeline)?;
        println!("{pipeline}");
        Ok(())
    }

    fn remote_inspect(&self, server: &str) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load()?.into_arc();
            HttpClient::new(config, server)
                .inspect(&self.pipeline)
                .await
                .map(|r| println!("{r}"))
        })
    }
}

impl BldCommand for InspectCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        match &self.server {
            Some(srv) => self.remote_inspect(srv),
            None => self.local_inspect(),
        }
    }
}
