use crate::base::{UnixSocketConnectionState, UnixSocketMessage, UnixSocketRead, UnixSocketState};
use crate::client::{UnixSocketServerReader, UnixSocketUnknownReader, UnixSocketWorkerReader};
use bld_core::workers::PipelineWorker;
use std::fs::remove_file;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::error;

pub struct UnixSocketNewStreamsListener {
    listener: UnixListener,
    server_tx: Sender<UnixSocketServerReader>,
    worker_tx: Sender<UnixSocketWorkerReader>,
    workers: Arc<Mutex<Vec<PipelineWorker>>>,
    readers: Arc<Mutex<Vec<UnixSocketUnknownReader>>>,
}

impl UnixSocketNewStreamsListener {
    pub fn bind<P>(
        path: P,
        server_tx: Sender<UnixSocketServerReader>,
        worker_tx: Sender<UnixSocketWorkerReader>,
        workers: Arc<Mutex<Vec<PipelineWorker>>>,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let _ = remove_file(&path);
        let listener = UnixListener::bind(path)?;
        Ok(Self {
            listener,
            server_tx,
            worker_tx,
            workers,
            readers: Arc::new(Mutex::new(vec![])),
        })
    }

    async fn receive(listener: UnixListener, readers: Arc<Mutex<Vec<UnixSocketUnknownReader>>>) {
        while let Ok((stream, _)) = listener.accept().await {
            let mut readers = readers.lock().await;
            readers.push(UnixSocketUnknownReader::new(stream));
        }
    }

    async fn read(
        server_tx: Sender<UnixSocketServerReader>,
        worker_tx: Sender<UnixSocketWorkerReader>,
        workers: Arc<Mutex<Vec<PipelineWorker>>>,
        readers: Arc<Mutex<Vec<UnixSocketUnknownReader>>>,
    ) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            let mut resolved_servers: Vec<usize> = vec![];
            let mut resolved_workers: Vec<(usize, u32)> = vec![];

            for (i, reader) in readers.iter_mut().enumerate() {
                match reader.try_read().await {
                    Ok(Some(messages)) => {
                        for message in messages.iter() {
                            match message {
                                UnixSocketMessage::ServerAck => {
                                    resolved_servers.push(i);
                                }
                                UnixSocketMessage::WorkerAck { pid } => {
                                    resolved_workers.push((i, *pid));
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(None) => reader.set_state(UnixSocketConnectionState::Stopped),
                    Err(e) => error!("could not read from unknown reader. {e}"),
                }
            }

            for i in resolved_servers {
                let reader = readers.remove(i);
                let _ = server_tx
                    .send(UnixSocketServerReader::new(reader.into()))
                    .await;
            }

            let mut workers = workers.lock().await;
            for (ri, pid) in resolved_workers {
                if let Some(wi) = workers.iter().position(|w| w.has_pid(pid)) {
                    let worker = workers.remove(wi);
                    let reader = readers.remove(ri);
                    let _ = worker_tx
                        .send(UnixSocketWorkerReader::new(worker, reader.into()))
                        .await;
                }
            }
        }
    }

    async fn cleanup(readers: Arc<Mutex<Vec<UnixSocketUnknownReader>>>) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            readers.retain(|r| !r.has_stopped());
        }
    }

    pub async fn listen(self) {
        let _ = tokio::join!(
            Self::receive(self.listener, self.readers.clone()),
            Self::read(
                self.server_tx,
                self.worker_tx,
                self.workers.clone(),
                self.readers.clone()
            ),
            Self::cleanup(self.readers.clone()),
        );
    }
}
