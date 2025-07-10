use actix::spawn;
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use std::path::PathBuf;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
    sync::{
        mpsc::{Receiver, Sender, channel},
        oneshot,
    },
};
use tracing::error;

#[derive(Debug)]
enum FileScannerMessage {
    Next(oneshot::Sender<Vec<String>>),
}

struct FileScannerBackend {
    path: PathBuf,
    file_handle: Option<File>,
    rx: Receiver<FileScannerMessage>,
}

impl FileScannerBackend {
    pub fn new(path: PathBuf, rx: Receiver<FileScannerMessage>) -> Self {
        Self {
            path,
            file_handle: None,
            rx,
        }
    }

    pub fn receive(self) {
        spawn(async move {
            self.receive_inner().await;
        });
    }

    async fn receive_inner(mut self) {
        while let Some(msg) = self.rx.recv().await {
            let res = match msg {
                FileScannerMessage::Next(resp_tx) => self.next(resp_tx).await,
            };
            if let Err(e) = res {
                error!("Message handling failed with {e}");
            }
        }
    }

    async fn try_file_handle(&mut self) -> Option<&mut File> {
        if self.file_handle.is_none() && self.path.is_file() {
            self.file_handle = File::open(&self.path).await.map(Some).unwrap_or(None);
        }
        self.file_handle.as_mut()
    }

    async fn next(&mut self, resp_tx: oneshot::Sender<Vec<String>>) -> Result<()> {
        let mut content: Vec<String> = vec![];
        let Some(file_handle) = self.try_file_handle().await else {
            resp_tx
                .send(content)
                .map_err(|_| anyhow!("oneshot response sender dropped"))?;
            return Ok(());
        };

        let reader = BufReader::new(file_handle);
        let mut lines = reader.lines();
        let mut next = lines.next_line().await?;
        while let Some(line) = next {
            content.push(line);
            next = lines.next_line().await?;
        }

        resp_tx
            .send(content)
            .map_err(|_| anyhow!("oneshot response sender dropped"))?;
        Ok(())
    }
}

pub struct FileScanner {
    tx: Sender<FileScannerMessage>,
}

impl FileScanner {
    pub fn new(config: &BldConfig, run_id: &str) -> Self {
        let path = config.log_full_path(run_id);
        let (tx, rx) = channel(4096);
        FileScannerBackend::new(path, rx).receive();
        Self { tx }
    }

    pub async fn scan(&self) -> Result<Vec<String>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(FileScannerMessage::Next(resp_tx)).await?;
        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
