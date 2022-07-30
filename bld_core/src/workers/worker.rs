use anyhow::{anyhow, bail};
use std::process::{Child, Command, ExitStatus};
use uuid::Uuid;

pub struct PipelineWorker {
    cmd: Command,
    child: Option<Child>,
    unix_client_id: Option<Uuid>,
}

impl PipelineWorker {
    pub fn new(cmd: Command) -> Self {
        Self {
            cmd,
            child: None,
            unix_client_id: None,
        }
    }

    pub fn get_pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }

    pub fn get_cid(&self) -> &Option<Uuid> {
        &self.unix_client_id 
    }

    pub fn set_cid(&mut self, cid: Uuid) {
        self.unix_client_id = Some(cid);
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        self.child.as_ref().map(|c| c.id() == pid) == Some(true)
    }

    pub fn has_cid(&self, cid: &Uuid) -> bool {
        self.unix_client_id.map(|id| id.to_string() == cid.to_string()).unwrap_or(false)
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
            Ok(Some(_)) => self
                .child
                .as_mut()
                .ok_or_else(|| anyhow!("worker has not spawned"))
                .and_then(|c| c.wait().map_err(|e| anyhow!(e))),
            _ => bail!("command is still running. cannot cleanup"),
        }
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.child
            .as_mut()
            .ok_or_else(|| anyhow!("worker child process not started"))
            .and_then(|c| c.kill().map(|_| ()).map_err(|e| anyhow!(e)))
    }
}
