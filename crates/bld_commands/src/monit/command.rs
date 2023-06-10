use crate::command::BldCommand;
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::{messages::MonitInfo, request::WebSocket};
use bld_sock::clients::MonitClient;
use clap::Args;
use futures::stream::StreamExt;
use tracing::debug;

#[derive(Args)]
#[command(about = "Connects to a bld server to monitor the execution of a pipeline")]
pub struct MonitCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'i',
        long = "pipeline-id",
        help = "The id of the pipeline to monitor. Takes precedence over pipeline"
    )]
    pipeline_id: Option<String>,

    #[arg(
        short = 'p',
        long = "pipeline",
        help = "The name of the pipeline of which to monitor the last run"
    )]
    pipeline: Option<String>,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to monitor the pipeline from"
    )]
    server: String,

    #[arg(
        long = "last",
        help = "Monitor the execution of the last invoked pipeline. Takes precedence over pipeline-id and pipeline"
    )]
    last: bool,
}

impl MonitCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server(&self.server)?;
        let url = format!("{}/ws-monit/", server.base_url_ws());

        debug!("establishing web socket connection on {}", url);

        let (_, framed) = WebSocket::new(&url)?
            .auth(server)
            .request()
            .connect()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        let (sink, stream) = framed.split();
        let addr = MonitClient::create(|ctx| {
            MonitClient::add_stream(stream, ctx);
            MonitClient::new(SinkWrite::new(sink, ctx))
        });

        debug!(
            "sending data over: {:?} {:?} {}",
            self.pipeline_id, self.pipeline, self.last
        );

        addr.send(MonitInfo::new(self.pipeline_id, self.pipeline, self.last))
            .await
            .map_err(|e| anyhow!(e))
    }
}

impl BldCommand for MonitCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let system = System::new();
        let result = system.block_on(self.request());

        system.run()?;
        result
    }
}
