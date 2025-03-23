use std::{collections::HashMap, sync::Arc};

use actix_web::rt::spawn;
use anyhow::{Result, anyhow};
use regex::Regex;
use tokio::sync::{
    mpsc::{Receiver, Sender, channel},
    oneshot,
};
use tracing::error;

#[derive(Debug)]
enum RegexCacheMessage {
    Get {
        key: String,
        resp_tx: oneshot::Sender<Option<Arc<Regex>>>,
    },
    Set {
        key: String,
        value: Arc<Regex>,
        resp_tx: oneshot::Sender<()>,
    },
}

struct RegexCacheBackend {
    cache: HashMap<String, Arc<Regex>>,
    rx: Receiver<RegexCacheMessage>,
}

impl RegexCacheBackend {
    pub fn new(rx: Receiver<RegexCacheMessage>) -> Self {
        Self {
            cache: HashMap::new(),
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
        while let Some(msg) = self.rx.recv().await {
            match msg {
                RegexCacheMessage::Get { key, resp_tx } => self.get(key, resp_tx)?,
                RegexCacheMessage::Set {
                    key,
                    value,
                    resp_tx,
                } => self.set(key, value, resp_tx)?,
            }
        }
        Ok(())
    }

    fn get(&mut self, key: String, resp_tx: oneshot::Sender<Option<Arc<Regex>>>) -> Result<()> {
        let value = self.cache.get(&key).cloned();
        resp_tx
            .send(value)
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }

    fn set(&mut self, key: String, value: Arc<Regex>, resp_tx: oneshot::Sender<()>) -> Result<()> {
        let _ = self.cache.insert(key, value);
        resp_tx
            .send(())
            .map_err(|_| anyhow!("oneshot response sender dropped"))
    }
}

pub struct RegexCache {
    tx: Sender<RegexCacheMessage>,
}

impl Default for RegexCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexCache {
    pub fn new() -> RegexCache {
        let (tx, rx) = channel(4096);
        RegexCacheBackend::new(rx).receive();
        Self { tx }
    }

    pub async fn get(&self, key: String) -> Result<Option<Arc<Regex>>> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(RegexCacheMessage::Get { key, resp_tx })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }

    pub async fn set(&self, key: String, value: Arc<Regex>) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(RegexCacheMessage::Set {
                key,
                value,
                resp_tx,
            })
            .await?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
