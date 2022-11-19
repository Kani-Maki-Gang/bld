use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use clap::Args;

#[derive(Args)]
#[command(about = "Start bld in server mode, listening to incoming build requests")]
pub struct ServerCommand {
    #[arg(short = 'H', long = "host", help = "The server's host address")]
    host: Option<String>,

    #[arg(short = 'P', long = "port", help = "The server's port")]
    port: Option<i64>,
}

impl BldCommand for ServerCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        let host = self.host.unwrap_or(config.local.server.host.to_owned());
        let port = self.port.unwrap_or(config.local.server.port);
        System::new().block_on(bld_server::start(config, host, port))
    }
}
