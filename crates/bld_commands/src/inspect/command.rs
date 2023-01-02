use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_utils::request::Request;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Inspects the contents of a pipeline on a bld server")]
pub struct InspectCommand {
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
        help = "The name of the server from which to inspect the pipeline"
    )]
    server: Option<String>,
}

impl BldCommand for InspectCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server_or_first(self.server.as_ref())?;
        let server_auth = config.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/inspect", server.host, server.port);
        let request = Request::post(&url).auth(server_auth);

        debug!("sending http request to {}", url);

        System::new().block_on(async move {
            request.send_json(&self.pipeline).await.map(|r: String| {
                println!("{r}");
            })
        })
    }
}
