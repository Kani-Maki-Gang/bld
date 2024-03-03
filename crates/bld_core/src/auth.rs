use std::{collections::HashMap, path::Path};

use actix_web::rt::spawn;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{create_dir_all, read_to_string, remove_file, File},
    io::AsyncWriteExt,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
};
use tracing::error;

enum LoginsMessage {
    Add(String, oneshot::Sender<String>),
    Remove(String),
    Code(String, String),
}

struct LoginsBackend {
    inner: HashMap<String, oneshot::Sender<String>>,
    rx: Receiver<LoginsMessage>,
}

impl LoginsBackend {
    pub fn new(rx: Receiver<LoginsMessage>) -> Self {
        Self {
            inner: HashMap::new(),
            rx,
        }
    }

    pub fn receive(self) {
        spawn(async move {
            if let Err(e) = self.receive_inner().await {
                error!("{e}");
            }
        });
    }

    async fn receive_inner(mut self) -> Result<()> {
        while let Some(message) = self.rx.recv().await {
            let result = match message {
                LoginsMessage::Add(token, sender) => self.add(token, sender),
                LoginsMessage::Remove(token) => self.remove(token),
                LoginsMessage::Code(token, code) => self.code(token, code),
            };
            if let Err(e) = result {
                error!("{e}");
            }
        }
        Ok(())
    }

    fn add(&mut self, token: String, sender: oneshot::Sender<String>) -> Result<()> {
        self.inner.insert(token, sender);
        Ok(())
    }

    fn remove(&mut self, token: String) -> Result<()> {
        self.inner.remove(&token);
        Ok(())
    }

    fn code(&mut self, token: String, code: String) -> Result<()> {
        if let Some(sender) = self.inner.remove(&token) {
            sender.send(code).map_err(|e| anyhow!(e))?;
        }
        Ok(())
    }
}

pub struct Logins {
    tx: Sender<LoginsMessage>,
}

impl Default for Logins {
    fn default() -> Self {
        let (tx, rx) = channel(4096);
        LoginsBackend::new(rx).receive();
        Self { tx }
    }
}

impl Logins {
    pub async fn add(&self, token: String, sender: oneshot::Sender<String>) -> Result<()> {
        self.tx
            .send(LoginsMessage::Add(token, sender))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn remove(&self, token: String) -> Result<()> {
        self.tx
            .send(LoginsMessage::Remove(token))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn code(&self, token: String, code: String) -> Result<()> {
        self.tx
            .send(LoginsMessage::Code(token, code))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

impl AuthTokens {
    pub fn new(access_token: String, refresh_token: Option<String>) -> Self {
        Self {
            access_token,
            refresh_token,
        }
    }
}

pub async fn read_tokens(path: &Path) -> Result<AuthTokens> {
    if !path.is_file() {
        bail!("file not found");
    }

    let content = read_to_string(path).await?;
    serde_json::from_str(&content).map_err(|e| anyhow!(e))
}

pub async fn write_tokens(path: &Path, tokens: AuthTokens) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).await?;
    }

    if path.is_file() {
        remove_file(path).await?;
    }

    let data = serde_json::to_vec(&tokens)?;
    File::create(path).await?.write_all(&data).await?;
    Ok(())
}
