use crate::client::UnixSocketClient;
use actix::System;
use anyhow::anyhow;
use bld_config::{path, BldConfig};
use bld_core::workers::PipelineWorker;
use std::env::temp_dir;
use std::fs::remove_file;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::sleep;
use tracing::{debug, error};

struct UnixSocketListener {
    listener: UnixListener,
    tx: Sender<UnixSocketClient>,
}

impl UnixSocketListener {
    pub fn new(cfg: Arc<BldConfig>, tx: Sender<UnixSocketClient>) -> anyhow::Result<Self> {
        debug!("starting listening to incoming unix socket clients");
        let path = path![temp_dir(), &cfg.local.unix_sock];
        let _ = remove_file(&path);
        let listener = UnixListener::bind(&path)
            .map_err(|e| anyhow!("Could not connect to unix socket. {e}"))?;
        Ok(Self { listener, tx })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        debug!("unix socket active for worker to connect");
        while let Ok((client, _)) = self.listener.accept().await {
            debug!("trying to listen to incoming unix socket clients");
            let _ = self
                .tx
                .send(UnixSocketClient::new(client))
                .await
                .map(|_| debug!("sent new client to handler"))
                .map_err(|e| {
                    error!("could not accept client from unix socket, {e}");
                    e
                });
        }
        Ok(())
    }

    pub async fn listen(cfg: Arc<BldConfig>, tx: Sender<UnixSocketClient>) {
        let listener = match Self::new(cfg, tx) {
            Ok(instance) => instance,
            Err(e) => {
                error!("{e}");
                return;
            }
        };
        if let Err(e) = listener.start().await {
            error!("{e}");
        }
    }
}

struct UnixSocketHandler {
    workers: Arc<Mutex<Vec<PipelineWorker>>>,
    clients: Vec<UnixSocketClient>,
    rx: Receiver<UnixSocketClient>,
}

impl UnixSocketHandler {
    pub fn new(workers: Arc<Mutex<Vec<PipelineWorker>>>, rx: Receiver<UnixSocketClient>) -> Self {
        Self {
            workers,
            clients: vec![],
            rx,
        }
    }

    fn recv(&mut self) {
        if let Ok(client) = self.rx.try_recv() {
            debug!("new unix socket client sent, adding it to the collection");
            self.clients.push(client);
        }
    }

    async fn read(&mut self) {
        for client in self.clients.iter_mut() {
            debug!("trying to read from unix socket client");
            match client.try_read().await {
                Ok(Some(messages)) => client.handle(messages),
                Ok(None) => client.stopped(),
                Err(e) => error!("could not read from unix socket client. {e}"),
            }
        }
    }

    fn cleanup(&mut self) {
        let mut workers = self.workers.lock().unwrap();
        for worker in workers.iter_mut() {
            if self
                .clients
                .iter()
                .find(|c| {
                    c.has_stopped() && worker.get_pid().map(|pid| c.has_pid(pid)).unwrap_or(false)
                })
                .is_some()
            {
                let _ = worker.cleanup();
            }
        }
        workers.retain(|w| !w.has_stopped());
        self.clients.retain(|c| !c.has_stopped());
    }

    pub async fn start(&mut self) {
        debug!("starting handling of incoming unix socket clients");
        loop {
            self.recv();
            self.read().await;
            self.cleanup();
            sleep(Duration::from_secs(1)).await;
        }
    }

    pub async fn handle(workers: Arc<Mutex<Vec<PipelineWorker>>>, rx: Receiver<UnixSocketClient>) {
        Self::new(workers, rx).start().await;
    }
}

pub fn start(config: Arc<BldConfig>, workers: Arc<Mutex<Vec<PipelineWorker>>>) {
    debug!("creating communication channel for listener and handler tasks");
    let (tx, rx) = channel(4096);
    let system = System::current();
    let arbiter = system.arbiter();

    // spawn listener that will send the bld unix socket client clients.
    arbiter.spawn(async move {
        UnixSocketListener::listen(config, tx).await;
    });

    // handle all incoming clients from the listener.
    arbiter.spawn(async move {
        UnixSocketHandler::handle(workers, rx).await;
    });
}
