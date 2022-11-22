use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::{bail, Result};
use bld_config::BldConfig;
use bld_supervisor::supervisor;
use clap::Args;
use tracing::{debug, error};

#[derive(Args)]
#[command(
    about = "Starts a bld supervisor that manages the pipeline worker queue. should be only invoked by the server"
)]
pub struct SupervisorCommand;

impl BldCommand for SupervisorCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;

        debug!("starting supervisor");

        System::new().block_on(async move {
            if let Err(e) = supervisor::start(config).await {
                error!("{e}");
                bail!("")
            }
            Ok(())
        })
    }
}
