use actix::spawn;
use anyhow::Result;
use async_signal::{Signal, Signals};
use bld_core::signals::{UnixSignals, UnixSignalsBackend};
use futures::stream::StreamExt;
use signal_hook::low_level;
use tokio::sync::mpsc::channel;
use tokio::task::JoinHandle;
use tracing::error;

pub struct CommandSignals {
    task: JoinHandle<()>,
}

impl CommandSignals {
    pub fn new() -> Result<(Self, UnixSignalsBackend)> {
        let (tx, rx) = channel(4096);

        let mut signals_tx = UnixSignals::new(tx);
        let signals_rx = UnixSignalsBackend::new(rx);

        let mut signals = if cfg!(target_family = "unix") {
            Signals::new([Signal::Term, Signal::Int, Signal::Quit])?
        } else {
            Signals::new([Signal::Int])?
        };

        let task = spawn(async move {
            while let Some(signal) = signals.next().await {
                let Ok(signal) = signal.map_err(|e| error!("{e}")) else {
                    return;
                };

                let result = match signal {
                    Signal::Int => {
                        let _ = signals_tx.sigint().await;
                        low_level::emulate_default_handler(Signal::Int as i32)
                    }
                    Signal::Term => {
                        let _ = signals_tx.sigterm().await;
                        low_level::emulate_default_handler(Signal::Term as i32)
                    }
                    Signal::Quit => {
                        let _ = signals_tx.sigquit().await;
                        low_level::emulate_default_handler(Signal::Quit as i32)
                    }
                    _ => Ok(()),
                };

                if let Err(e) = result {
                    error!("{e}");
                }
            }
        });

        let instance = Self { task };

        Ok((instance, signals_rx))
    }

    pub async fn stop(self) -> Result<()> {
        self.task.abort();
        Ok(())
    }
}
