use crate::base::{
    Queue, UnixSocketConnectionState, UnixSocketHandle, UnixSocketRead, UnixSocketState,
};
use crate::client::UnixSocketWorkerReader;
use crate::queues::WorkerQueue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;
use tracing::{debug, error};

type SyncMutex<T> = std::sync::Mutex<T>;
type AsyncMutex<T> = tokio::sync::Mutex<T>;

pub struct UnixSocketWorkersListener {
    queue: Arc<SyncMutex<WorkerQueue>>,
    readers: Arc<AsyncMutex<Vec<UnixSocketWorkerReader>>>,
    rx: Receiver<UnixSocketWorkerReader>,
}

impl UnixSocketWorkersListener {
    pub fn new(queue: Arc<SyncMutex<WorkerQueue>>, rx: Receiver<UnixSocketWorkerReader>) -> Self {
        Self {
            queue,
            readers: Arc::new(AsyncMutex::new(vec![])),
            rx,
        }
    }

    async fn receive(
        mut rx: Receiver<UnixSocketWorkerReader>,
        readers: Arc<AsyncMutex<Vec<UnixSocketWorkerReader>>>,
    ) {
        loop {
            if let Some(reader) = rx.recv().await {
                let mut readers = readers.lock().await;
                readers.push(reader);
            }
        }
    }

    async fn read(
        queue: Arc<SyncMutex<WorkerQueue>>,
        readers: Arc<AsyncMutex<Vec<UnixSocketWorkerReader>>>,
    ) {
        loop {
            sleep(Duration::from_millis(300)).await;
            let mut readers = readers.lock().await;
            for reader in readers.iter_mut() {
                match reader.try_read().await {
                    Ok(Some(messages)) => reader.handle(queue.clone(), messages),
                    Ok(None) => reader.set_state(UnixSocketConnectionState::Stopped),
                    Err(e) => error!("could not read from worker reader. {e}"),
                }
            }
        }
    }

    async fn cleanup(
        queue: Arc<SyncMutex<WorkerQueue>>,
        readers: Arc<AsyncMutex<Vec<UnixSocketWorkerReader>>>,
    ) {
        loop {
            sleep(Duration::from_millis(300)).await;
            let mut pids = vec![];
            {
                let mut readers = readers.lock().await;
                let len = readers.len();
                readers.retain(|r| {
                    let stopped = r.has_stopped();
                    if stopped {
                        pids.push(r.get_worker_pid());
                    }
                    !stopped
                });
                if len - readers.len() > 0 {
                    debug!("cleaning up {} readers", len - readers.len());
                }
            }
            let mut queue = queue.lock().unwrap();
            queue.dequeue(&pids);
        }
    }

    pub async fn listen(self) {
        let _ = tokio::join!(
            Self::receive(self.rx, self.readers.clone()),
            Self::read(self.queue.clone(), self.readers.clone()),
            Self::cleanup(self.queue, self.readers.clone())
        );
    }
}
