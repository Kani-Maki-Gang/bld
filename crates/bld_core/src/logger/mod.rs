use actix_web::rt::spawn;
use anyhow::Result;
use bld_config::{path, BldConfig};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::sync::mpsc::{channel, Receiver, Sender};

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

    async fn receive(mut self, mut rx: Receiver<LoggerMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                LoggerMessage::Write(txt) => self.write(&txt),
                LoggerMessage::WriteLine(txt) => self.write_line(&txt),
                LoggerMessage::Info(txt) => self.info(&txt),
                LoggerMessage::InfoLine(txt) => self.info_line(&txt),
                LoggerMessage::Error(txt) => self.error(&txt),
                LoggerMessage::ErrorLine(txt) => self.error_line(&txt),
            }
        }
    }

    pub fn write(&mut self, text: &str) {
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
    }

    pub fn write_line(&mut self, text: &str) {
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
    }

    pub fn info(&mut self, text: &str) {
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
    }

    pub fn info_line(&mut self, text: &str) {
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
    }

    pub fn error(&mut self, text: &str) {
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
    }

    pub fn error_line(&mut self, text: &str) {
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
    }
}

#[derive(Debug)]
enum LoggerMessage {
    Write(String),
    WriteLine(String),
    Info(String),
    InfoLine(String),
    Error(String),
    ErrorLine(String),
}

#[derive(Default)]
pub struct LoggerSender {
    tx: Option<Sender<LoggerMessage>>,
}

impl LoggerSender {
    pub fn shell() -> Self {
        let (tx, rx) = channel(4096);
        let logger = LoggerReceiver::shell();

        spawn(async move { logger.receive(rx).await });

        Self { tx: Some(tx) }
    }

    pub fn file(config: Arc<BldConfig>, run_id: &str) -> Result<Self> {
        let (tx, rx) = channel(4096);
        let logger = LoggerReceiver::file(config, run_id)?;

        spawn(async move { logger.receive(rx).await });

        Ok(Self { tx: Some(tx) })
    }

    pub async fn write(&self, txt: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(LoggerMessage::Write(txt)).await?;
        }
        Ok(())
    }

    pub async fn write_line(&self, txt: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(LoggerMessage::WriteLine(txt)).await?;
        }
        Ok(())
    }

    pub async fn info(&self, txt: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(LoggerMessage::Info(txt)).await?;
        }
        Ok(())
    }

    pub async fn info_line(&self, txt: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(LoggerMessage::InfoLine(txt)).await?;
        }
        Ok(())
    }

    pub async fn error(&self, txt: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(LoggerMessage::Error(txt)).await?;
        }
        Ok(())
    }

    pub async fn error_line(&self, txt: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(LoggerMessage::ErrorLine(txt)).await?;
        }
        Ok(())
    }
}
