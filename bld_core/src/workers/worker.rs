use anyhow::{anyhow, Result};
use std::process::{Child, Command, ExitStatus};

pub struct PipelineWorker {
    run_id: String,
    cmd: Command,
    child: Option<Child>,
}

impl PipelineWorker {
    pub fn new(run_id: String, cmd: Command) -> Self {
        Self {
            run_id,
            cmd,
            child: None,
        }
    }

    pub fn get_run_id(&self) -> &str {
        &self.run_id
    }

    pub fn get_pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        self.child.as_ref().map(|c| c.id() == pid).unwrap_or(false)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child
            .as_mut()
            .ok_or_else(|| anyhow!("worker has not spawned"))
            .and_then(|c| c.try_wait().map_err(|e| anyhow!(e)))
    }

    pub fn spawn(&mut self) -> Result<()> {
        self.child = Some(self.cmd.spawn().map_err(|e| anyhow!(e))?);
        Ok(())
    }

    pub fn completed(&mut self) -> bool {
        self.try_wait().is_ok()
    }

    pub fn cleanup(&mut self) -> Result<ExitStatus> {
        self.try_wait()
            .map_err(|_| anyhow!("command is still running. cannot cleanup"))
            .and_then(|_| {
                self.child
                    .as_mut()
                    .ok_or_else(|| anyhow!("worker has not spawned"))
                    .and_then(|c| c.wait().map_err(|e| anyhow!(e)))
            })
    }
}
