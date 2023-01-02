use anyhow::{Result, anyhow};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum UnixSignalMessage {
    SIGINT,
    SIGTERM,
}

pub struct UnixSignalsSender {
    tx: Sender<UnixSignalMessage>,
}

impl UnixSignalsSender {
    pub fn new(tx: Sender<UnixSignalMessage>) -> Self {
        Self { tx }
    }

    pub async fn sigint(&mut self) -> Result<()> {
        self.tx.send(UnixSignalMessage::SIGINT).await?;
        Ok(())
    }

    pub async fn sigterm(&mut self) -> Result<()> {
        self.tx.send(UnixSignalMessage::SIGTERM).await?;
        Ok(())
    }
}

pub struct UnixSignalsReceiver {
    rx: Receiver<UnixSignalMessage>,
}

impl UnixSignalsReceiver {
    pub fn new(rx: Receiver<UnixSignalMessage>) -> Self {
        Self { rx }
    }

    pub fn try_next(&mut self) -> Result<UnixSignalMessage> {
        self.rx.try_recv().map_err(|e| anyhow!(e))
    }
}
