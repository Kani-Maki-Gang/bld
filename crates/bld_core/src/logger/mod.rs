use actix_web::rt::spawn;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use std::{fmt::Write as FmtWrite, io::Write, sync::Arc};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
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
    TryRetrieveOutput {
        resp_tx: oneshot::Sender<String>,
    },
}

enum LoggerReceiver {
    Shell,
    File { handle: File },
    InMemory { output: String },
}

impl LoggerReceiver {
    pub fn shell() -> Self {
        Self::Shell
    }

    pub async fn file(config: Arc<BldConfig>, run_id: &str) -> Result<Self> {
        let path = config.log_full_path(run_id);
        Ok(Self::File {
            handle: if path.is_file() {
                File::open(&path).await?
            } else {
                File::create(&path).await?
            },
        })
    }

    pub fn in_memory() -> Self {
        Self::InMemory {
            output: String::new(),
        }
    }

    async fn receive(mut self, mut rx: Receiver<LoggerMessage>) -> Result<()> {
        while let Some(msg) = rx.recv().await {
            match msg {
                LoggerMessage::Write { text, resp_tx } => self.write(&text, resp_tx).await?,
                LoggerMessage::WriteLine { text, resp_tx } => {
                    self.write_line(&text, resp_tx).await?
                }
                LoggerMessage::Info { text, resp_tx } => self.info(&text, resp_tx).await?,
                LoggerMessage::InfoLine { text, resp_tx } => self.info_line(&text, resp_tx).await?,
                LoggerMessage::Error { text, resp_tx } => self.error(&text, resp_tx).await?,
                LoggerMessage::ErrorLine { text, resp_tx } => {
                    self.error_line(&text, resp_tx).await?
                }
                LoggerMessage::TryRetrieveOutput { resp_tx } => {
                    self.try_retrieve_output(resp_tx).await?
                }
            }
        }
        Ok(())
    }

    pub async fn write(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                print!("{}", text);
            }
            Self::File { handle } => {
                let bytes = text.as_bytes();
                handle.write(&bytes).await?;
            }
            Self::InMemory { output } => {
                write!(output, "{text}")?;
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub async fn write_line(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                println!("{text}");
            }
            Self::File { handle } => {
                let text = format!("{text}\n");
                let bytes = text.as_bytes();
                handle.write(&bytes).await?;
            }
            Self::InMemory { output } => {
                writeln!(output, "{text}")?;
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub async fn info(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stdout = StandardStream::stdout(ColorChoice::Always);
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = write!(&mut stdout, "{text}");
                let _ = stdout.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                let bytes = text.as_bytes();
                handle.write(&bytes).await?;
            }
            Self::InMemory { output } => {
                write!(output, "{text}")?;
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub async fn info_line(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stdout = StandardStream::stdout(ColorChoice::Always);
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = writeln!(&mut stdout, "{text}");
                let _ = stdout.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                let text = format!("{text}\n");
                let bytes = text.as_bytes();
                handle.write(&bytes).await?;
            }
            Self::InMemory { output } => {
                writeln!(output, "{text}")?;
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    pub async fn error(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stderr = StandardStream::stderr(ColorChoice::Always);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                let _ = write!(&mut stderr, "{text}");
                let _ = stderr.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                let bytes = text.as_bytes();
                handle.write(&bytes).await?;
            }
            Self::InMemory { output } => {
                write!(output, "{text}")?;
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot rsponse sender dropped"))
    }

    pub async fn error_line(&mut self, text: &str, resp_tx: oneshot::Sender<()>) -> Result<()> {
        match self {
            Self::Shell => {
                let mut stderr = StandardStream::stderr(ColorChoice::Always);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                let _ = writeln!(&mut stderr, "{text}");
                let _ = stderr.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                let text = format!("{text}\n");
                let bytes = text.as_bytes();
                handle.write(&bytes).await?;
            }
            Self::InMemory { output } => {
                writeln!(output, "{text}")?;
            }
        }

        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    async fn try_retrieve_output(&mut self, resp_tx: oneshot::Sender<String>) -> Result<()> {
        let output = match self {
            Self::Shell => String::new(),
            Self::File { handle } => {
                let mut output = String::new();
                handle.read_to_string(&mut output).await?;
                output
            }
            Self::InMemory { output } => output.clone(),
        };

        resp_tx
            .send(output)
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }
}

pub struct LoggerSender {
    tx: Sender<LoggerMessage>,
}

impl Default for LoggerSender {
    fn default() -> Self {
        Self::shell()
    }
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

        Self { tx }
    }

    pub async fn file(config: Arc<BldConfig>, run_id: &str) -> Result<Self> {
        let (tx, rx) = channel(4096);
        let logger = LoggerReceiver::file(config, run_id).await?;

        spawn(async move { logger.receive(rx).await });

        Ok(Self { tx })
    }

    pub fn in_memory() -> Self {
        let (tx, rx) = channel(4096);
        let logger = LoggerReceiver::in_memory();

        spawn(async move { logger.receive(rx).await });

        Self { tx }
    }

    pub async fn write(&self, text: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx.send(LoggerMessage::Write { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn write_line(&self, text: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(LoggerMessage::WriteLine { text, resp_tx })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn write_seperator(&self) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(LoggerMessage::WriteLine {
                text: format!("{:-<1$}", "", 80),
                resp_tx,
            })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn info(&self, text: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx.send(LoggerMessage::Info { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn info_line(&self, text: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(LoggerMessage::InfoLine { text, resp_tx })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn error(&self, text: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx.send(LoggerMessage::Error { text, resp_tx }).await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn error_line(&self, text: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(LoggerMessage::ErrorLine { text, resp_tx })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn try_retrieve_output(&self) -> Result<String> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(LoggerMessage::TryRetrieveOutput { resp_tx })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
