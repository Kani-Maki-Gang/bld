use crate::message::UnixSocketMessage;
use anyhow::{anyhow, bail};
use std::path::Path;
use tokio::net::UnixStream;
use tracing::error;
use uuid::Uuid;

pub struct UnixSocketClient {
    pub id: Uuid,
    stream: UnixStream,
}

impl UnixSocketClient {
    pub async fn connect<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            stream: UnixStream::connect(path).await?,
        })
    }

    pub fn new(stream: UnixStream) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            stream,
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
}
