use std::{fs::File, io::Write, path::PathBuf, sync::Arc};

use actix_web::rt::spawn;
use anyhow::{anyhow, Result};
use bld_config::{path, BldConfig};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};
use tracing::error;

#[derive(Debug)]
enum LoggerMessage {
    Write {
        text: String,
        resp_tx: oneshot::Sender<()>,
    },
    WriteLine {
        text: String,
        resp_tx: oneshot::Sender<()>,
    },
    Info {
        text: String,
        resp_tx: oneshot::Sender<()>,
    },
    InfoLine {
        text: String,
        resp_tx: oneshot::Sender<()>,
    },
    Error {
        text: String,
        resp_tx: oneshot::Sender<()>,
    },
    ErrorLine {
        text: String,
        resp_tx: oneshot::Sender<()>,
    },
}

enum LoggerReceiver {
    Shell,
    File { handle: File },
}

impl LoggerReceiver {
    pub fn shell() -> Self {
        Self::Shell
    }

    pub fn file(config: Arc<BldConfig>, run_id: &str) -> Result<Self> {
        let path = path![&config.local.logs, run_id];
        Ok(Self::File {
            handle: if path.is_file() {
                File::open(&path)?
            } else {
                File::create(&path)?
            },
        })
    }

    async fn receive(mut self, mut rx: Receiver<LoggerMessage>) -> Result<()> {
        while let Some(msg) = rx.recv().await {
            match msg {
                LoggerMessage::Write { text, resp_tx } => self.write(&text, resp_tx)?,
                LoggerMessage::WriteLine { text, resp_tx } => self.write_line(&text, resp_tx)?,
                LoggerMessage::Info { text, resp_tx } => self.info(&text, resp_tx)?,
                LoggerMessage::InfoLine { text, resp_tx } => self.info_line(&text, resp_tx)?,
                LoggerMessage::Error { text, resp_tx } => self.error(&text, resp_tx)?,
                LoggerMessage::ErrorLine { text, resp_tx } => self.error_line(&text, resp_tx)?,
            }
        }
        Ok(())
    }

    pub fn write(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                print!("{}", text);
            }
            Self::File { handle } => {
                if let Err(e) = write!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub fn write_line(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                println!("{text}");
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub fn info(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stdout = StandardStream::stdout(ColorChoice::Always);
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = write!(&mut stdout, "{text}");
                let _ = stdout.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                if let Err(e) = write!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub fn info_line(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stdout = StandardStream::stdout(ColorChoice::Always);
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = writeln!(&mut stdout, "{text}");
                let _ = stdout.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub fn error(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stderr = StandardStream::stderr(ColorChoice::Always);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                let _ = write!(&mut stderr, "{text}");
                let _ = stderr.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                if let Err(e) = write!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot rsponse sender dropped"))
    }

    pub fn error_line(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stderr = StandardStream::stderr(ColorChoice::Always);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                let _ = writeln!(&mut stderr, "{text}");
                let _ = stderr.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }
}

#[derive(Default)]
pub struct LoggerSender {
    tx: Option<Sender<LoggerMessage>>,
}

impl LoggerSender {
    pub fn shell() -> Self {
        let (tx, rx) = channel(4096);
        let logger = LoggerReceiver::shell();

        spawn(async move {
            if let Err(e) = logger.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx: Some(tx) }
    }

    pub fn file(config: Arc<BldConfig>, run_id: &str) -> Result<Self> {
        let (tx, rx) = channel(4096);
        let logger = LoggerReceiver::file(config, run_id)?;

        spawn(async move { logger.receive(rx).await });

        Ok(Self { tx: Some(tx) })
    }

    pub async fn write(&self, text: String) -> Result<()> {
        let Some(tx) = &self.tx else {
            return Ok(())
        };

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::Write { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn write_line(&self, text: String) -> Result<()> {
        let Some(tx) = &self.tx else {return Ok(())};

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::WriteLine { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn write_seperator(&self) -> Result<()> {
        let Some(tx) = &self.tx else {return Ok(())};

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::WriteLine {
            text: format!("{:-<1$}", "", 80),
            resp_tx,
        })
        .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn info(&self, text: String) -> Result<()> {
        let Some(tx) = &self.tx else {return Ok(())};

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::Info { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn info_line(&self, text: String) -> Result<()> {
        let Some(tx) = &self.tx else {return Ok(())};

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::InfoLine { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn error(&self, text: String) -> Result<()> {
        let Some(tx) = &self.tx else {return Ok(())};

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::Error { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn error_line(&self, text: String) -> Result<()> {
        let Some(tx) = &self.tx else {return Ok(())};

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(LoggerMessage::ErrorLine { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
