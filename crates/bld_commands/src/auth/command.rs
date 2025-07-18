use std::sync::Arc;

use actix::{Actor, StreamHandler, System, io::SinkWrite};
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use bld_http::WebSocket;
use bld_models::dtos::LoginClientMessage;
use bld_sock::LoginClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use futures::stream::StreamExt;
use tracing::debug;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Initiates the login process for a bld server")]
pub struct AuthCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to login into"
    )]
    server: String,
}

impl AuthCommand {
    async fn login(config: Arc<BldConfig>, server: String) -> Result<()> {
        let server = config.server(&server)?;
        let auth_path = config.auth_full_path(&server.name);
        let url = format!("{}/v1/ws-login/", server.base_url_ws());

        debug!("establishing web socket connection on {}", url);

        let (_, framed) = WebSocket::new(&url)?
            .auth(&auth_path)
            .await
            .request()
            .connect()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        let (sink, stream) = framed.split();
        let addr = LoginClient::create(|ctx| {
            LoginClient::add_stream(stream, ctx);
            LoginClient::new(
                config.clone(),
                server.name.to_owned(),
                SinkWrite::new(sink, ctx),
            )
        });

        addr.send(LoginClientMessage::Init)
            .await
            .map_err(|e| anyhow!(e))
    }
}

impl BldCommand for AuthCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let system = System::new();
        let res = system.block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let server = self.server.to_owned();

            debug!("running login subcommand with --server: {}", self.server);

            Self::login(config, server).await
        });

        let _ = system.run();
        res
    }
}
