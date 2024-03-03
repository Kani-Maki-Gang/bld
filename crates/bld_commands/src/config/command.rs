use crate::command::BldCommand;
use actix::System;
use anyhow::Result;
use bld_config::{BldConfig, BldLocalConfig, BldRemoteServerConfig};
use bld_core::fs::FileSystem;
use bld_utils::{sync::IntoArc, term};
use clap::Args;
use tracing::metadata::LevelFilter;

#[derive(Args)]
#[command(about = "List or edit bld's configuration")]
pub struct ConfigCommand {
    #[arg(long = "edit", short = 'e', help = "Edit the config file")]
    pub edit: bool,
}

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

    async fn list_all() -> Result<()> {
        let config = BldConfig::load().await?;
        Self::list_locals(&config.local)?;
        Self::list_remote(&config.remote)
    }

    async fn edit() -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let fs = FileSystem::local(config);
        fs.edit_config().await
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
        System::new().block_on(async move {
            if self.edit {
                Self::edit().await
            } else {
                Self::list_all().await
            }
        })
    }
}
