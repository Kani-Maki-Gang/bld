use crate::base::UnixSocketWrite;
use std::fs::remove_file;
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
        let _ = remove_file(&path);
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
