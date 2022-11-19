use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_utils::request::Request;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Lists information of pipelines in a bld server")]
pub struct ListCommand {
    #[arg(short = 's', long = "server", help = "The name of the server from which to fetch pipeline information")]
    server: Option<String>,
}

impl BldCommand for ListCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(self.server.as_ref())?;
        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/list", server.host, server.port);
        let request = Request::get(&url).auth(&server_auth);

        debug!("sending request to {}", url);

        System::new().block_on(async move {
            request
                .send()
                .await
                .map(|r: String| println!("{r}"))
        })
    }
}
