use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_utils::request::Request;
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

impl BldCommand for RemoveCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.remote.server_or_first(self.server.as_ref())?;

        debug!(
            "running remove subcommand with --server: {} and --pipeline: {}",
            server.name, self.pipeline
        );

        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/remove", server.host, server.port);
        let request = Request::post(&url).auth(server_auth);

        System::new().block_on(async move {
            debug!("sending request to {}", url);
            request.send_json(self.pipeline).await.map(|r: String| {
                println!("{r}");
            })
        })
    }
}
