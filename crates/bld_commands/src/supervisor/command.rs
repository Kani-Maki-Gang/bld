use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_supervisor::supervisor;
use clap::{ArgMatches, Command};
use tracing::debug;

static SUPERVISOR: &str = "supervisor";

pub struct SupervisorCommand;

impl BldCommand for SupervisorCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        SUPERVISOR
    }

    fn interface(&self) -> Command {
        Command::new(SUPERVISOR)
            .about("Starts a bld supervisor that manages the pipeline worker queue. should be only invoked by the server")
            .version(VERSION)
    }

    fn exec(&self, _matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        debug!("starting supervisor");
        System::new().block_on(async move { supervisor::start(config).await })
    }
}
