use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_utils::request::Request;
use clap::Args;

#[derive(Args)]
#[command(about = "Stops a running pipeline on a server")]
pub struct StopCommand {
    #[arg(
        short = 'i',
        long = "id",
        required = true,
        help = "The id of a pipeline running on a server"
    )]
    pipeline_id: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server that the pipeline is running"
    )]
    server: Option<String>,
}

impl BldCommand for StopCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;

        let server = config.remote.server_or_first(self.server.as_ref())?;

        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/stop", server.host, server.port);

        System::new().block_on(async move {
            Request::post(&url)
                .auth(server_auth)
                .send_json(self.pipeline_id)
                .await
                .map(|r: String| {
                    println!("{r}");
                })
        })
    }
}
