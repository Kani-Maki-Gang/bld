use crate::base::UnixSocketMessage;
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use tokio::net::UnixStream;

#[async_trait]
pub trait UnixSocketWrite {
    fn get_stream(&self) -> &UnixStream;

    async fn try_write(&self, message: &UnixSocketMessage) -> anyhow::Result<()> {
        let stream = self.get_stream();
        if let Err(e) = stream.writable().await {
            bail!("socket is not writable. {e}");
        }

        message
            .to_bytes()
            .and_then(|d| stream.try_write(&d).map_err(|e| anyhow!(e)))
            .map(|_| ())
    }
}
