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
pub struct SupervisorCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,
}

impl BldCommand for SupervisorCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?;
            debug!("starting supervisor");
            if let Err(e) = supervisor::start(config).await {
                error!("{e}");
                bail!("")
            }
            Ok(())
        })
    }
}
