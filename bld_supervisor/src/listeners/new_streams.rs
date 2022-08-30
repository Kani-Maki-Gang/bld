use crate::base::{
    Queue, UnixSocketConnectionState, UnixSocketMessage, UnixSocketRead, UnixSocketState,
};
use crate::client::{UnixSocketServerReader, UnixSocketUnknownReader, UnixSocketWorkerReader};
use crate::queues::WorkerQueue;
use std::fs::remove_file;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::{debug, error};
use uuid::Uuid;

type SyncMutex<T> = std::sync::Mutex<T>;
type AsyncMutex<T> = tokio::sync::Mutex<T>;

pub struct UnixSocketNewStreamsListener {
    listener: UnixListener,
    queue: Arc<SyncMutex<WorkerQueue>>,
    server_tx: Sender<UnixSocketServerReader>,
    worker_tx: Sender<UnixSocketWorkerReader>,
    readers: Arc<AsyncMutex<Vec<UnixSocketUnknownReader>>>,
}

impl UnixSocketNewStreamsListener {
    pub fn bind<P>(
        path: P,
        queue: Arc<SyncMutex<WorkerQueue>>,
        server_tx: Sender<UnixSocketServerReader>,
        worker_tx: Sender<UnixSocketWorkerReader>,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let _ = remove_file(&path);
        let listener = UnixListener::bind(path)?;
        Ok(Self {
            listener,
            queue,
            server_tx,
            worker_tx,
            readers: Arc::new(AsyncMutex::new(vec![])),
        })
    }

    async fn receive(
        listener: UnixListener,
        readers: Arc<AsyncMutex<Vec<UnixSocketUnknownReader>>>,
    ) {
        while let Ok((stream, _)) = listener.accept().await {
            let mut readers = readers.lock().await;
            readers.push(UnixSocketUnknownReader::new(stream));
            debug!("accepted new stream. adding it to readers");
        }
    }

    async fn read(
        queue: Arc<SyncMutex<WorkerQueue>>,
        server_tx: Sender<UnixSocketServerReader>,
        worker_tx: Sender<UnixSocketWorkerReader>,
        readers: Arc<AsyncMutex<Vec<UnixSocketUnknownReader>>>,
    ) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            let mut resolved_servers: Vec<Uuid> = vec![];
            let mut resolved_workers: Vec<(Uuid, u32)> = vec![];

            for reader in readers.iter_mut() {
                debug!("iterating new stream readers");
                match reader.try_read().await {
                    Ok(Some(messages)) => {
                        for message in messages.iter() {
                            match message {
                                UnixSocketMessage::ServerAck => {
                                    debug!("message ServerAck was sent");
                                    resolved_servers.push(reader.get_id());
                                }
                                UnixSocketMessage::WorkerAck { pid } => {
                                    debug!("message WorkerAck ({pid}) was sent");
                                    resolved_workers.push((reader.get_id(), *pid));
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("stream has closed from the other end");
                        reader.set_state(UnixSocketConnectionState::Stopped)
                    }
                    Err(e) => error!("could not read from unknown reader. {e}"),
                }
            }

            for id in resolved_servers {
                if let Some(idx) = readers.iter().position(|r| r.get_id() == id) {
                    let reader = readers.remove(idx);
                    let _ = server_tx
                        .send(UnixSocketServerReader::new(reader.into()))
                        .await;
                    debug!("transfering stream to the server handling thread");
                }
            }

            let mut queue = queue.lock().unwrap();
            for (rid, pid) in resolved_workers {
                if queue.contains(pid) {
                    if let Some(idx) = readers.iter().position(|r| r.get_id() == rid) {
                        let reader = readers.remove(idx);
                        let _ = worker_tx
                            .send(UnixSocketWorkerReader::new(pid, reader.into()))
                            .await;
                        debug!("transfering stream to the worker handling thread");
                    }
                }
            }
        }
    }

    async fn cleanup(readers: Arc<AsyncMutex<Vec<UnixSocketUnknownReader>>>) {
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
                self.queue,
                self.server_tx,
                self.worker_tx,
                self.readers.clone()
            ),
            Self::cleanup(self.readers.clone()),
        );
    }
}
