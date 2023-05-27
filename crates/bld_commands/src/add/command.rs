use anyhow::Result;
use bld_config::definitions::DEFAULT_V2_PIPELINE_CONTENT;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::sync::IntoArc;
use clap::Args;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Creates a new pipeline")]
pub struct AddCommand {
    #[arg(
        short = 'p',
        long = "pipeline",
        help = "The path to the new pipeline file"
    )]
    pipeline: String,

    #[arg(
        short = 'e',
        long = "edit",
        help = "Edit the pipeline file immediatelly after creation"
    )]
    edit: bool,
}

impl BldCommand for AddCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config.clone());

        proxy.create(&self.pipeline, DEFAULT_V2_PIPELINE_CONTENT)?;

        if self.edit {
            proxy.edit(&self.pipeline)?;
        }

        Ok(())
    }
}
