use crate::command::BldCommand;
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_sock::clients::MonitClient;
use bld_sock::messages::MonitInfo;
use bld_utils::request::WebSocket;
use clap::Args;
use futures::stream::StreamExt;
use tracing::debug;

#[derive(Args)]
#[command(about = "Connects to a bld server to monitor the execution of a pipeline")]
pub struct MonitCommand {
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
        help = "The name of the server to monitor"
    )]
    server: Option<String>,

    #[arg(
        long = "last",
        help = "Monitor the execution of the last invoked pipeline. Takes precedence over pipeline-id and pipeline"
    )]
    last: bool,
}

impl MonitCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server_or_first(self.server.as_ref())?;
        let server_auth = config.same_auth_as(server)?.to_owned();
        let url = format!(
            "{}://{}:{}/ws-monit/",
            server.ws_protocol(),
            server.host,
            server.port
        );

        debug!("establishing web socket connection on {}", url);

        let (_, framed) = WebSocket::new(&url)?
            .auth(&server_auth)
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
    fn exec(self) -> Result<()> {
        let system = System::new();
        let result = system.block_on(self.request());

        system.run()?;
        result
    }
}
