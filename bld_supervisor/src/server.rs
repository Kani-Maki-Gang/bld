use crate::listeners::{
    UnixSocketNewStreamsListener, UnixSocketServersListener, UnixSocketWorkersListener,
};
use crate::queues::WorkerQueue;
use bld_config::{path, BldConfig};
use std::env::temp_dir;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;
use tracing::debug;

pub async fn start(config: BldConfig) -> anyhow::Result<()> {
    debug!("creating communication channel for listener tasks");

    let sock_path = path![temp_dir(), config.local.unix_sock];
    let queue = Arc::new(Mutex::new(WorkerQueue::new(20)));
    let workers = Arc::new(Mutex::new(vec![]));
    let (server_tx, server_rx) = channel(4096);
    let (worker_tx, worker_rx) = channel(4096);

    let new_streams_listener =
        UnixSocketNewStreamsListener::bind(sock_path, server_tx, worker_tx, workers.clone())?;
    let servers_listener = UnixSocketServersListener::new(queue.clone(), server_rx);
    let workers_listener = UnixSocketWorkersListener::new(queue.clone(), worker_rx);

    tokio::join!(
        new_streams_listener.listen(),
        servers_listener.listen(),
        workers_listener.listen()
    );

    Ok(())
}
