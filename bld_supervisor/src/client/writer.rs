use crate::base::UnixSocketWrite;
use std::path::Path;
use tokio::net::UnixStream;

pub struct UnixSocketWriter {
    stream: UnixStream,
}

impl UnixSocketWriter {
    pub async fn connect<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            stream: UnixStream::connect(path).await?,
        })
    }
}

impl UnixSocketWrite for UnixSocketWriter {
    fn get_stream(&self) -> &UnixStream {
        &self.stream
    }
}
