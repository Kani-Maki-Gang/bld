use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use clap::Args;
use tracing::metadata::LevelFilter;

#[derive(Args)]
#[command(about = "Start bld in server mode, listening to incoming build requests")]
pub struct ServerCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(short = 'H', long = "host", help = "The server's host address")]
    host: Option<String>,

    #[arg(short = 'P', long = "port", help = "The server's port")]
    port: Option<i64>,
}

impl BldCommand for ServerCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn tracing_level(&self) -> LevelFilter {
        if self.verbose() {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        }
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?;
            let host = self
                .host
                .unwrap_or_else(|| config.local.server.host.to_owned());
            let port = self.port.unwrap_or(config.local.server.port);
            bld_server::start(config, host, port).await
        })
    }
}
