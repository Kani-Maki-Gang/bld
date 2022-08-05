use crate::base::{UnixSocketConnectionState, UnixSocketHandle, UnixSocketRead, UnixSocketState};
use crate::client::UnixSocketWorkerReader;
use crate::queues::WorkerQueue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::error;

pub struct UnixSocketWorkersListener {
    queue: Arc<Mutex<WorkerQueue>>,
    readers: Arc<Mutex<Vec<UnixSocketWorkerReader>>>,
    rx: Receiver<UnixSocketWorkerReader>,
}

impl UnixSocketWorkersListener {
    pub fn new(queue: Arc<Mutex<WorkerQueue>>, rx: Receiver<UnixSocketWorkerReader>) -> Self {
        Self {
            queue,
            readers: Arc::new(Mutex::new(vec![])),
            rx,
        }
    }

    async fn receive(
        mut rx: Receiver<UnixSocketWorkerReader>,
        readers: Arc<Mutex<Vec<UnixSocketWorkerReader>>>,
    ) {
        loop {
            if let Some(reader) = rx.recv().await {
                let mut readers = readers.lock().await;
                readers.push(reader);
            }
        }
    }

    async fn read(readers: Arc<Mutex<Vec<UnixSocketWorkerReader>>>) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            for reader in readers.iter_mut() {
                match reader.try_read().await {
                    Ok(Some(messages)) => reader.handle(messages),
                    Ok(None) => reader.set_state(UnixSocketConnectionState::Stopped),
                    Err(e) => error!("could not read from worker reader. {e}"),
                }
            }
        }
    }

    async fn cleanup(
        queue: Arc<Mutex<WorkerQueue>>,
        readers: Arc<Mutex<Vec<UnixSocketWorkerReader>>>,
    ) {
        loop {
            sleep(Duration::from_millis(300)).await;

            {
                let mut readers = readers.lock().await;
                readers.retain(|r| !r.has_stopped());
            }
            let mut queue = queue.lock().await;
            queue.refresh();
        }
    }

    pub async fn listen(self) {
        let _ = tokio::join!(
            Self::receive(self.rx, self.readers.clone()),
            Self::read(self.readers.clone()),
            Self::cleanup(self.queue, self.readers.clone())
        );
    }
}
