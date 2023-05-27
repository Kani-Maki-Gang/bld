use anyhow::Result;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::sync::IntoArc;
use clap::Args;

use crate::command::BldCommand;

#[derive(Args)]
#[command(about = "Edit a pipeline file")]
pub struct EditCommand {
    #[arg(short = 'p', long = "pipline", help = "The name of the pipeline file")]
    pipeline: String
}

impl BldCommand for EditCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config);
        proxy.edit(&self.pipeline)
    }
}
