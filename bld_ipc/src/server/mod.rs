use anyhow::{anyhow, bail};
use crate::messages::UnixSocketMessage;
use tokio::io::Interest;
use tokio::net::UnixStream;
use std::path::Path;

pub struct BldUnixSocketServer {
    unix_stream: UnixStream,
}

impl BldUnixSocketServer {
    pub async fn connect<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path> 
    {
        Ok(Self {
            unix_stream: UnixStream::connect(path).await?,
        })
    }

    pub async fn try_read(&self) -> anyhow::Result<UnixSocketMessage> {
        let ready = self.unix_stream.ready(Interest::READABLE | Interest::WRITABLE).await?;

        if !ready.is_readable() {
            bail!("Socket is not readable");
        }

        let mut data = vec![0; 4096];
        self.unix_stream
            .try_read(&mut data)
            .map_err(|e| anyhow!(e))
            .and_then(|n| serde_json::from_slice::<UnixSocketMessage>(&data[0..n]).map_err(|e| anyhow!(e)))
    }
}
