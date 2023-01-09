use actix::spawn;
use anyhow::Result;
use bld_core::signals::{UnixSignalsReceiver, UnixSignalsSender};
use futures::stream::StreamExt;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook_tokio::{Handle, Signals};
use tokio::sync::mpsc::channel;
use tokio::task::JoinHandle;

pub struct CommandSignals {
    handle: Handle,
    task: JoinHandle<()>,
}

impl CommandSignals {
    pub fn new() -> Result<(Self, UnixSignalsReceiver)> {
        let (tx, rx) = channel(4096);

        let mut signals_tx = UnixSignalsSender::new(tx);
        let signals_rx = UnixSignalsReceiver::new(rx);

        let mut signals = Signals::new([SIGINT, SIGTERM])?;
        let handle = signals.handle();

        let task = spawn(async move {
            while let Some(signal) = signals.next().await {
                let _ = match signal {
                    SIGINT => signals_tx.sigint().await,
                    SIGTERM => signals_tx.sigterm().await,
                    _ => Ok(()),
                };
            }
        });

        let instance = Self { handle, task };

        Ok((instance, signals_rx))
    }

    pub async fn stop(self) -> Result<()> {
        self.handle.close();
        self.task.await?;
        Ok(())
    }
}
