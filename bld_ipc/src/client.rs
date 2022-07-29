use crate::message::UnixSocketMessage;
use anyhow::{anyhow, bail};
use std::mem::size_of;
use std::path::Path;
use tokio::net::UnixStream;
use tracing::error;
use uuid::Uuid;

pub struct UnixSocketClient {
    pub id: Uuid,
    unix_stream: UnixStream,
}

impl UnixSocketClient {
    fn get_message_bytes(message: &UnixSocketMessage) -> anyhow::Result<Vec<u8>> {
        let json_bytes = serde_json::to_vec(message).map_err(|e| anyhow!(e))?;
        let len: i32 = json_bytes.len().try_into()?;
        let mut bytes = vec![];
        for byte in len.to_le_bytes() {
            bytes.push(byte);
        }
        for byte in json_bytes {
            bytes.push(byte);
        }
        Ok(bytes)
    }

    fn get_messages_from_bytes(
        source: &mut &[u8],
        capacity: usize,
    ) -> anyhow::Result<Option<Vec<UnixSocketMessage>>> {
        if capacity == 0 {
            return Ok(None);
        }
        let mut current = 0;
        let mut messages = vec![];
        while current < capacity {
            let i32_size = size_of::<i32>();
            let (message_size_bytes, rest) = source.split_at(i32_size);
            *source = rest;

            let message_size: usize =
                i32::from_le_bytes(message_size_bytes.try_into()?).try_into()?;
            let (message_bytes, rest) = source.split_at(message_size);
            *source = rest;

            messages.push(serde_json::from_slice(message_bytes).map_err(|e| anyhow!(e))?);

            current += i32_size + message_size;
        }
        Ok(Some(messages))
    }

    pub async fn connect<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            unix_stream: UnixStream::connect(path).await?,
        })
    }

    pub fn new(unix_stream: UnixStream) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            unix_stream,
        }
    }

    pub async fn try_read(&self) -> anyhow::Result<Option<Vec<UnixSocketMessage>>> {
        self.unix_stream.readable().await?;

        let mut data = [0u8; 4096];
        self.unix_stream
            .try_read(&mut data)
            .map_err(|e| anyhow!(e))
            .and_then(|n| Self::get_messages_from_bytes(&mut &data[..], n))
    }

    pub async fn try_write(&self, value: &UnixSocketMessage) -> anyhow::Result<()> {
        if let Err(e) = self.unix_stream.writable().await {
            error!("{e}");
            bail!("Socket is not writable");
        }

        Self::get_message_bytes(value)
            .and_then(|d| self.unix_stream.try_write(&d).map_err(|e| anyhow!(e)))
            .map(|_| ())
    }
}
