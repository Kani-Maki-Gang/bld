use anyhow::{anyhow, bail};
use std::process::{Child, Command, ExitStatus};

pub struct PipelineWorker {
    cmd: Command,
    child: Option<Child>,
}

impl PipelineWorker {
    pub fn new(cmd: Command) -> Self {
        Self { cmd, child: None }
    }

    fn try_wait(&mut self) -> anyhow::Result<Option<ExitStatus>> {
        self.child
            .as_mut()
            .ok_or_else(|| anyhow!("worker has not spawned"))
            .and_then(|c| c.try_wait().map_err(|e| anyhow!(e)))
    }

    pub fn spawn(&mut self) -> anyhow::Result<()> {
        self.child = Some(self.cmd.spawn().map_err(|e| anyhow!(e))?);
        Ok(())
    }

    pub fn completed(&mut self) -> bool {
        self.try_wait().is_ok()
    }

    pub fn cleanup(&mut self) -> anyhow::Result<ExitStatus> {
        match self.try_wait() {
            Ok(Some(_)) => {
                self.child
                    .as_mut()
                    .ok_or_else(|| anyhow!("worker has not spawned"))
                    .and_then(|c| c.wait().map_err(|e| anyhow!(e)))
            }
            _ => bail!("command is still running. cannot cleanup"),
        }
    }
}

