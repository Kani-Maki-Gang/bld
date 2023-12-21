use std::process::ExitStatus;

use anyhow::{anyhow, Result};
use tokio::process::{Child, Command};

#[cfg(target_family = "unix")]
use nix::{
    sys::signal::{self, Signal},
    unistd::Pid,
};

#[derive(Debug)]
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

    pub fn has_run_id(&self, run_id: &str) -> bool {
        self.run_id == run_id
    }

    pub fn get_pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id()).unwrap_or(None)
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        self.get_pid().map(|id| id == pid).unwrap_or(false)
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

    pub async fn cleanup(&mut self) -> Result<ExitStatus> {
        self.try_wait()
            .map_err(|_| anyhow!("command is still running. cannot cleanup"))?;
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow!("worker has not spawned"))?;
        child.wait().await.map_err(|e| anyhow!(e))
    }

    #[cfg(target_family = "unix")]
    pub async fn stop(&mut self) -> Result<()> {
        let pid = self
            .get_pid()
            .ok_or_else(|| anyhow!("child instance doesnt have a pid"))?;
        signal::kill(Pid::from_raw(pid.try_into()?), Signal::SIGTERM)?;
        Ok(())
    }

    #[cfg(target_family = "windows")]
    pub async fn stop(&mut self) -> Result<()> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow!("worker has not spawned"))?;
        child.kill().await.map_err(|e| anyhow!(e))
    }
}
