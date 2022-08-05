use crate::BldCommand;
use actix_web::rt::System;
use bld_config::{definitions::VERSION, BldConfig};
use clap::{App as ClapApp, ArgMatches, SubCommand};
use tracing::debug;

static SUPERVISOR: &str = "supervisor";

pub struct SupervisorCommand;

impl SupervisorCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for SupervisorCommand {
    fn id(&self) -> &'static str {
        SUPERVISOR
    }

    fn interface(&self) -> ClapApp<'static, 'static> {
        SubCommand::with_name(SUPERVISOR)
            .about("Starts a bld supervisor that manages the pipeline worker queue. should be only invoked by the server")
            .version(VERSION)
    }

    fn exec(&self, _matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let _config = BldConfig::load()?;
        debug!("starting supervisor");
        System::new().block_on(async move {});
        Ok(())
    }
}
