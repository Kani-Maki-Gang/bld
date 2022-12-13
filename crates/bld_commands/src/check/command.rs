use crate::command::BldCommand;
use anyhow::Result;
use bld_config::definitions::TOOL_DEFAULT_PIPELINE_FILE;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::{Load, Yaml};
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Checks a pipeline file for errors")]
pub struct CheckCommand {
    #[arg(short = 'p', long = "pipeline", default_value = TOOL_DEFAULT_PIPELINE_FILE, help = "Path to pipeline script")]
    pipeline: String,
}

impl BldCommand for CheckCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::Local.into_arc();
        let content = proxy.read(&self.pipeline)?;
        let pipeline = Yaml::load_with_verbose_errors(&content)?;
        pipeline.validate_with_verbose_errors(config, proxy)
    }
}
