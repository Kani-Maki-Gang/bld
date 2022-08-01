use crate::message::UnixSocketMessage;
use anyhow::{anyhow, bail};
use std::path::Path;
use tokio::net::UnixStream;
use tracing::{debug, error};
use uuid::Uuid;

pub enum UnixSocketClientStatus {
    Active,
    Stopped,
}

pub struct UnixSocketClient {
    pub id: Uuid,
    stream: UnixStream,
    status: UnixSocketClientStatus,
    worker_pid: Option<u32>,
}

impl UnixSocketClient {
    pub async fn connect<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            stream: UnixStream::connect(path).await?,
            status: UnixSocketClientStatus::Active,
            worker_pid: None,
        })
    }

    pub fn new(stream: UnixStream) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            stream,
            status: UnixSocketClientStatus::Active,
            worker_pid: None,
        }
    }

    pub fn get_pid(&self) -> Option<&u32> {
        self.worker_pid.as_ref()
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        self.worker_pid.map(|id| id == pid).unwrap_or(false)
    }

    pub fn has_stopped(&self) -> bool {
        match self.status {
            UnixSocketClientStatus::Stopped => true,
            _ => false,
        }
    }

    pub async fn try_read(&self) -> anyhow::Result<Option<Vec<UnixSocketMessage>>> {
        self.stream.readable().await?;

        let mut data = [0u8; 4096];
        self.stream
            .try_read(&mut data)
            .map_err(|e| anyhow!(e))
            .and_then(|n| UnixSocketMessage::from_bytes(&mut &data[..], n))
    }

    pub async fn try_write(&self, message: &UnixSocketMessage) -> anyhow::Result<()> {
        if let Err(e) = self.stream.writable().await {
            error!("{e}");
            bail!("Socket is not writable");
        }

        message
            .to_bytes()
            .and_then(|d| self.stream.try_write(&d).map_err(|e| anyhow!(e)))
            .map(|_| ())
    }

    pub fn handle(&mut self, messages: Vec<UnixSocketMessage>) {
        for message in messages.iter() {
            match message {
                UnixSocketMessage::Ping { pid } => {
                    debug!(
                        "worker with pid: {pid} sent PING message from unix socket with id: {}",
                        self.id
                    );
                    self.worker_pid = Some(*pid);
                }
                UnixSocketMessage::Exit { pid } => {
                    debug!(
                        "worker with pid: {pid} sent EXIT message from unix socket with id: {}",
                        self.id
                    );
                    self.status = UnixSocketClientStatus::Stopped;
                }
            }
        }
    }

    pub fn stopped(&mut self) {
        debug!(
            "worker client with id: {} has closed without EXIT message.",
            self.id
        );
        self.status = UnixSocketClientStatus::Stopped;
    }
}
