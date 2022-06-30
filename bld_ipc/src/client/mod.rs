use crate::messages::UnixSocketMessage;
use anyhow::{anyhow, bail};
use tokio::io::Interest;
use tokio::net::UnixStream;
use std::path::Path;

pub struct BldUnixSocketClient {
    unix_stream: UnixStream,
}

impl BldUnixSocketClient {
    pub async fn connect<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path> 
    {
        Ok(Self {
            unix_stream: UnixStream::connect(path).await?,
        })
    }

    pub async fn try_write(&self, value: &UnixSocketMessage) -> anyhow::Result<()> {
        let ready = self.unix_stream.ready(Interest::READABLE | Interest::WRITABLE).await?;

        if !ready.is_writable() {
            bail!("Socket is not writable");
        }

        let data = serde_json::to_vec(value).map_err(|e| anyhow!(e))?;
        self.unix_stream
            .try_write(&data)
            .map(|_| ())
            .map_err(|e| anyhow!(e))
    }
}
