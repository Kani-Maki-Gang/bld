use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_sock::LoginClient;
use clap::Args;
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

impl BldCommand for AuthCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?;
            let logger = Logger::shell();
            debug!("running login subcommand with --server: {}", self.server);
            LoginClient::connect(config, logger, self.server)
                .await?
                .run()
                .await
        })
    }
}
