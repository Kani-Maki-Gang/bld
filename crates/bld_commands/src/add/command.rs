use std::fmt::Write;
use std::process::{Command, ExitStatus};

use anyhow::{bail, Result};
use bld_config::definitions::DEFAULT_V2_PIPELINE_CONTENT;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
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

impl AddCommand {
    fn create(&self) -> Result<()> {
        let proxy = PipelineFileSystemProxy::Local;
        let path = proxy.path(&self.pipeline)?;
        if !path.is_file() {
            proxy.create(&self.pipeline, DEFAULT_V2_PIPELINE_CONTENT)?;
            println!("Pipeline file '{}' created successfully", self.pipeline);
            Ok(())
        } else {
            bail!("Pipeline already exists")
        }
    }

    fn edit(&self, config: &BldConfig) -> Result<()> {
        if self.edit {
            let proxy = PipelineFileSystemProxy::Local;
            let path = proxy.path(&self.pipeline)?;

            let mut edit_command = Command::new("/bin/bash");
            edit_command.args(["-c", &format!("{} {}", config.local.editor, path.display())]);

            let process_status = edit_command.status()?;
            if !ExitStatus::success(&process_status) {
                let mut error = String::new();
                let process = edit_command.output()?;
                writeln!(error, "editor process finished with {}", process.status)?;
                write!(error, "{}", String::from_utf8_lossy(&process.stderr))?;
                bail!(error);
            }
        }
        Ok(())
    }
}

impl BldCommand for AddCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        self.create()?;
        self.edit(&config)
    }
}
