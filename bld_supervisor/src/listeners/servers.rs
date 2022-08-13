use crate::base::{UnixSocketConnectionState, UnixSocketHandle, UnixSocketRead, UnixSocketState};
use crate::client::UnixSocketServerReader;
use crate::queues::WorkerQueue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;
use tracing::{debug, error};

type SyncMutex<T> = std::sync::Mutex<T>;
type AsyncMutex<T> = tokio::sync::Mutex<T>;

pub struct UnixSocketServersListener {
    queue: Arc<SyncMutex<WorkerQueue>>,
    readers: Arc<AsyncMutex<Vec<UnixSocketServerReader>>>,
    rx: Receiver<UnixSocketServerReader>,
}

impl UnixSocketServersListener {
    pub fn new(queue: Arc<SyncMutex<WorkerQueue>>, rx: Receiver<UnixSocketServerReader>) -> Self {
        Self {
            queue,
            readers: Arc::new(AsyncMutex::new(vec![])),
            rx,
        }
    }

    async fn receive(
        mut rx: Receiver<UnixSocketServerReader>,
        readers: Arc<AsyncMutex<Vec<UnixSocketServerReader>>>,
    ) {
        loop {
            if let Some(reader) = rx.recv().await {
                let mut readers = readers.lock().await;
                readers.push(reader);
                debug!("received new server reader");
            }
        }
    }

    async fn read(
        queue: Arc<SyncMutex<WorkerQueue>>,
        readers: Arc<AsyncMutex<Vec<UnixSocketServerReader>>>,
    ) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            for reader in readers.iter_mut() {
                match reader.try_read().await {
                    Ok(Some(messages)) => reader.handle(queue.clone(), messages),
                    Ok(None) => reader.set_state(UnixSocketConnectionState::Stopped),
                    Err(e) => error!("could not read from server reader. {e}"),
                }
            }
        }
    }

    async fn cleanup(readers: Arc<AsyncMutex<Vec<UnixSocketServerReader>>>) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            readers.retain(|r| !r.has_stopped());
        }
    }

    pub async fn listen(self) {
        let _ = tokio::join!(
            Self::receive(self.rx, self.readers.clone()),
            Self::read(self.queue, self.readers.clone()),
            Self::cleanup(self.readers.clone())
        );
    }
}
