use crate::base::UnixSocketMessage;
use anyhow::anyhow;
use async_trait::async_trait;
use tokio::net::UnixStream;

#[async_trait]
pub trait UnixSocketRead {
    fn set_leftover(&mut self, leftover: Option<Vec<u8>>);

    fn get_leftover(&self) -> Option<&Vec<u8>>;

    fn get_stream(&self) -> &UnixStream;

    async fn try_read(&mut self) -> anyhow::Result<Option<Vec<UnixSocketMessage>>> {
        let stream = self.get_stream();
        stream.readable().await?;

        let mut data = [0u8; 4096];
        let (messages, leftover) = stream
            .try_read(&mut data)
            .map_err(|e| anyhow!(e))
            .and_then(|n| {
                UnixSocketMessage::from_bytes(
                    &mut &data[..],
                    self.get_leftover().map(|l| &l[..]),
                    n,
                )
            })?;

        self.set_leftover(leftover);

        Ok(messages)
    }
}
