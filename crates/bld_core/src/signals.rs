use anyhow::{Result, anyhow};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

#[derive(Debug)]
pub enum UnixSignal {
    SIGINT,
    SIGTERM,
    SIGQUIT,
}

pub struct UnixSignalMessage {
    pub signal: UnixSignal,
    pub resp_tx: oneshot::Sender<()>,
}

impl UnixSignalMessage {
    pub fn new(signal: UnixSignal, resp_tx: oneshot::Sender<()>) -> Self {
        Self { signal, resp_tx }
    }
}

pub struct UnixSignals {
    tx: Sender<UnixSignalMessage>,
}

impl UnixSignals {
    pub fn new(tx: Sender<UnixSignalMessage>) -> Self {
        Self { tx }
    }

    pub async fn sigint(&mut self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(UnixSignalMessage::new(UnixSignal::SIGINT, resp_tx))
            .await?;
        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn sigterm(&mut self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(UnixSignalMessage::new(UnixSignal::SIGTERM, resp_tx))
            .await?;
        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn sigquit(&mut self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(UnixSignalMessage::new(UnixSignal::SIGQUIT, resp_tx))
            .await?;
        resp_rx.await.map_err(|e| anyhow!(e))
    }
}

pub struct UnixSignalsBackend {
    rx: Receiver<UnixSignalMessage>,
}

impl UnixSignalsBackend {
    pub fn new(rx: Receiver<UnixSignalMessage>) -> Self {
        Self { rx }
    }

    pub fn try_next(&mut self) -> Result<UnixSignalMessage> {
        self.rx.try_recv().map_err(|e| anyhow!(e))
    }
}
