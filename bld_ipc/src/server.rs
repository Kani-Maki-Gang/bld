use crate::client::UnixSocketClient;
use crate::message::UnixSocketMessage;
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
use uuid::Uuid;

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

    fn set_worker_cid(&self, pid: u32, cid: Uuid) {
        let mut workers = self.workers.lock().unwrap();
        if let Some(worker) = workers.iter_mut().find(|w| w.has_pid(pid)) {
            worker.set_cid(cid);
        }
    }

    fn remove_worker(&self, cid: &Uuid) -> anyhow::Result<()> {
        let mut workers = self.workers.lock().unwrap();
        let idx = workers.iter().position(|w| w.has_cid(cid)).ok_or_else(|| {
            anyhow!("could not remove worker with cid: {cid} because it was not found")
        })?;
        workers
            .iter_mut()
            .nth(idx)
            .ok_or_else(|| {
                anyhow!("could not remove worker with cid: {cid} because it was not found")
            })
            .and_then(|w| w.cleanup())?;
        workers.remove(idx);
        debug!("removed worker with cid: {cid}");
        Ok(())
    }

    fn remove_client(&mut self, cid: &Uuid) -> anyhow::Result<()> {
        let i = self
            .clients
            .iter()
            .position(|s| s.id == *cid)
            .ok_or_else(|| {
                anyhow!("could not remove client with cid: {cid} because it was not found")
            })?;
        self.clients.remove(i);
        debug!("removed client with cid: {cid}");
        Ok(())
    }

    fn messages(
        &self,
        client: &UnixSocketClient,
        messages: Vec<UnixSocketMessage>,
        removals: &mut Vec<Uuid>,
    ) {
        for message in messages.iter() {
            match message {
                UnixSocketMessage::Ping { pid } => {
                    debug!(
                        "worker with pid: {pid} sent PING message from unix socket with id: {}",
                        client.id
                    );
                    self.set_worker_cid(*pid, client.id);
                }
                UnixSocketMessage::Exit { pid } => {
                    debug!(
                        "worker with pid: {pid} sent EXIT message from unix socket with id: {}",
                        client.id
                    );
                    removals.push(client.id);
                }
            }
        }
    }

    fn closed_stream(client: &UnixSocketClient, removals: &mut Vec<Uuid>) {
        debug!(
            "worker client with id: {} has closed without EXIT message.",
            client.id
        );
        removals.push(client.id);
    }

    fn recv(&mut self) {
        if let Ok(client) = self.rx.try_recv() {
            debug!("new unix socket client sent, adding it to the collection");
            self.clients.push(client);
        }
    }

    async fn read(&mut self, removals: &mut Vec<Uuid>) {
        for client in self.clients.iter() {
            debug!("trying to read from unix socket client");
            match client.try_read().await {
                Ok(Some(messages)) => self.messages(client, messages, removals),
                Ok(None) => Self::closed_stream(client, removals),
                Err(e) => error!("could not read from unix socket client. {e}"),
            }
        }
    }

    fn cleanup(&mut self, removals: &mut Vec<Uuid>) {
        removals.retain(|cid| {
            let result = self
                .remove_worker(cid)
                .and_then(|_| self.remove_client(cid));
            !result.is_ok()
        })
    }

    pub async fn start(&mut self) {
        debug!("starting handling of incoming unix socket clients");
        let mut removals = vec![];
        loop {
            self.recv();
            self.read(&mut removals).await;
            self.cleanup(&mut removals);
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
