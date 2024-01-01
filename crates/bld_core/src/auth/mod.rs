use std::{collections::HashMap, path::Path};

use actix_web::rt::spawn;
use anyhow::{anyhow, bail, Result};
use serde::{Serialize, Deserialize};
use tokio::{
    fs::{create_dir_all, read_to_string, remove_file, File},
    io::AsyncWriteExt,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
};
use tracing::error;

enum LoginProcessMessage {
    Add(String, oneshot::Sender<String>),
    Remove(String),
    Code(String, String),
}

#[derive(Default)]
struct LoginProcessReceiver {
    inner: HashMap<String, oneshot::Sender<String>>,
}

impl LoginProcessReceiver {
    pub async fn receive(mut self, mut rx: Receiver<LoginProcessMessage>) -> Result<()> {
        while let Some(message) = rx.recv().await {
            let result = match message {
                LoginProcessMessage::Add(token, sender) => self.add(token, sender),
                LoginProcessMessage::Remove(token) => self.remove(token),
                LoginProcessMessage::Code(token, code) => self.code(token, code),
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

pub struct LoginProcess {
    tx: Sender<LoginProcessMessage>,
}

impl Default for LoginProcess {
    fn default() -> Self {
        let (tx, rx) = channel(4096);

        spawn(async move {
            let receiver = LoginProcessReceiver::default();
            if let Err(e) = receiver.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx }
    }
}

impl LoginProcess {
    pub async fn add(&self, token: String, sender: oneshot::Sender<String>) -> Result<()> {
        self.tx
            .send(LoginProcessMessage::Add(token, sender))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn remove(&self, token: String) -> Result<()> {
        self.tx
            .send(LoginProcessMessage::Remove(token))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn code(&self, token: String, code: String) -> Result<()> {
        self.tx
            .send(LoginProcessMessage::Code(token, code))
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenParams {
    pub refresh_token: String,
}

impl RefreshTokenParams {
    pub fn new(refresh_token: &str) -> Self {
        Self {
            refresh_token: refresh_token.to_owned(),
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
