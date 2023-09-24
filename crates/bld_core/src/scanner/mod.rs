use actix::spawn;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
};
use tracing::error;

enum FileScannerMessage {
    Next(oneshot::Sender<Vec<String>>),
}

struct FileScannerReceiver {
    path: PathBuf,
    file_handle: Option<File>,
    receiver: Receiver<FileScannerMessage>,
}

impl FileScannerReceiver {
    pub fn new(
        config: Arc<BldConfig>,
        run_id: &str,
        receiver: Receiver<FileScannerMessage>,
    ) -> Self {
        Self {
            path: config.log_full_path(run_id),
            file_handle: None,
            receiver,
        }
    }

    pub async fn receive(&mut self) -> Result<()> {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                FileScannerMessage::Next(resp_tx) => self.next(resp_tx).await?,
            }
        }
        Ok(())
    }

    async fn try_open(&mut self) {
        if self.file_handle.is_some() {
            return;
        }
        self.file_handle = match self.path.is_file() {
            true => File::open(&self.path).await.map(Some).unwrap_or(None),
            false => None,
        };
    }

    async fn next(&mut self, resp_tx: oneshot::Sender<Vec<String>>) -> Result<()> {
        self.try_open().await;

        let mut content: Vec<String> = vec![];
        let Some(file_handle) = self.file_handle.as_mut() else {
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
    pub fn new(config: Arc<BldConfig>, run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        let mut receiver = FileScannerReceiver::new(config, run_id, rx);

        spawn(async move {
            if let Err(e) = receiver.receive().await {
                error!("{e}");
            }
        });

        Self { tx }
    }

    pub async fn scan(&mut self) -> Result<Vec<String>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(FileScannerMessage::Next(resp_tx)).await?;
        resp_rx.await.map_err(|e| anyhow!(e))
    }
}
