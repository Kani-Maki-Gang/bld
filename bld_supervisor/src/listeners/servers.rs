use crate::queues::WorkerQueue;
use crate::base::{UnixSocketConnectionState, UnixSocketHandle, UnixSocketRead, UnixSocketState};
use crate::client::UnixSocketServerReader;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::error;

pub struct UnixSocketServersListener {
    queue: Arc<Mutex<WorkerQueue>>,
    readers: Arc<Mutex<Vec<UnixSocketServerReader>>>,
    rx: Receiver<UnixSocketServerReader>,
}

impl UnixSocketServersListener {
    pub fn new(queue: Arc<Mutex<WorkerQueue>>, rx: Receiver<UnixSocketServerReader>) -> Self {
        Self {
            queue,
            readers: Arc::new(Mutex::new(vec![])),
            rx,
        }
    }

    async fn receive(
        mut rx: Receiver<UnixSocketServerReader>,
        readers: Arc<Mutex<Vec<UnixSocketServerReader>>>,
    ) {
        loop {
            if let Some(reader) = rx.recv().await {
                let mut readers = readers.lock().await;
                readers.push(reader);
            }
        }
    }

    async fn read(queue: Arc<Mutex<WorkerQueue>>, readers: Arc<Mutex<Vec<UnixSocketServerReader>>>) {
        loop {
            sleep(Duration::from_millis(300)).await;

            let mut readers = readers.lock().await;
            for reader in readers.iter_mut() {
                match reader.try_read().await {
                    Ok(Some(messages)) => reader.handle(messages),
                    Ok(None) => reader.set_state(UnixSocketConnectionState::Stopped),
                    Err(e) => error!("could not read from server reader. {e}"),
                }
            }
        }
    }

    async fn cleanup(readers: Arc<Mutex<Vec<UnixSocketServerReader>>>) {
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
