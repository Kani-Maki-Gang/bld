use crate::command::BldCommand;
use anyhow::Result;
use bld_config::{BldConfig, BldLocalConfig, BldRemoteServerConfig};
use bld_utils::term;
use clap::Args;
use tracing::metadata::LevelFilter;

#[derive(Args)]
#[command(about = "Lists bld's configuration")]
pub struct ConfigCommand;

impl ConfigCommand {
    fn list_locals(local: &BldLocalConfig) -> Result<()> {
        term::print_info("Local configuration:")?;
        println!("{}", serde_yaml::to_string(local)?);
        Ok(())
    }

    fn list_remote(remote: &[BldRemoteServerConfig]) -> Result<()> {
        term::print_info("Remote configuration:")?;
        println!("{}", serde_yaml::to_string(remote)?);
        Ok(())
    }

    fn list_all(config: &BldConfig) -> Result<()> {
        Self::list_locals(&config.local)?;
        println!();
        Self::list_remote(&config.remote)?;
        Ok(())
    }
}

impl BldCommand for ConfigCommand {
    fn verbose(&self) -> bool {
        false
    }

    fn tracing_level(&self) -> LevelFilter {
        LevelFilter::OFF
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        Self::list_all(&config)
    }
}
