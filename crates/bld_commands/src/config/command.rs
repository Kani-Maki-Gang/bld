use crate::command::BldCommand;
use anyhow::Result;
use bld_config::{BldConfig, BldLocalConfig, BldRemoteServerConfig};
use bld_core::proxies::PipelineFileSystemProxy;
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

    fn list_all() -> Result<()> {
        let config = BldConfig::load()?;
        Self::list_locals(&config.local)?;
        Self::list_remote(&config.remote)
    }

    fn edit() -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.edit_config()
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
        if self.edit {
            Self::edit()
        } else {
            Self::list_all()
        }
    }
}
