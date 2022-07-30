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

    fn try_assign_cid_to_worker(&self, pid: u32, cid: Uuid) {
        let mut workers = self.workers.lock().unwrap();
        if let Some(worker) = workers.iter_mut().find(|w| w.has_pid(pid)) {
            worker.set_cid(cid);
        }
    }

    fn try_remove_worker_by_pid(&self, pid: u32) {
        let mut workers = self.workers.lock().unwrap();
        if let Some(worker) = workers.iter_mut().find(|w| w.has_pid(pid)) {
            match worker.cleanup() {
                Ok(_) => debug!("cleaned up worker with pid: {:?} and cid: {:?}", worker.get_pid(), worker.get_cid()),
                Err(e) => error!("{e}"),
            }
            let idx = workers.iter().position(|w| w.has_pid(pid)).unwrap();
            workers.remove(idx);
        }
    }

    fn try_remove_worker_by_cid(&self, cid: &Uuid) {
        let mut workers = self.workers.lock().unwrap();
        debug!("removing worker client with id: {cid}");
        if let Some(worker) = workers.iter_mut().find(|w| w.has_cid(cid)) {
            match worker.cleanup() {
                Ok(_) => debug!("clean up worker with pid: {:?} and cid: {:?}", worker.get_pid(), worker.get_cid()),
                Err(e) => error!("{e}"),
            }
            let idx = workers.iter().position(|w| w.has_cid(cid)).unwrap();
            workers.remove(idx);
        }
    }

    fn remove_clients_by_ids(&mut self, ids: &[Uuid]) {
        for id in ids {
            let idx = self.clients.iter().position(|s| s.id == *id);
            if let Some(i) = idx {
                self.try_remove_worker_by_cid(id);
                self.clients.remove(i);
            }
        }
    }

    fn handle_messages(
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
                    self.try_assign_cid_to_worker(*pid, client.id);
                }
                UnixSocketMessage::Exit { pid } => {
                    debug!(
                        "worker with pid: {pid} sent EXIT message from unix socket with id: {}",
                        client.id
                    );
                    self.try_remove_worker_by_pid(*pid);
                    removals.push(client.id);
                }
            }
        }
    }

    fn handle_empty(&self, client: &UnixSocketClient, removals: &mut Vec<Uuid>) {
        debug!(
            "worker client with id: {} has closed without EXIT message.",
            client.id
        );
        removals.push(client.id);
    }

    pub async fn start(&mut self) {
        debug!("starting handling of incoming unix socket clients");
        loop {
            if let Ok(client) = self.rx.try_recv() {
                debug!("new unix socket client sent, adding it to the collection");
                self.clients.push(client);
            }
            let mut removals = vec![];
            for client in self.clients.iter() {
                debug!("trying to read from unix socket client");
                match client.try_read().await {
                    Ok(Some(messages)) => self.handle_messages(client, messages, &mut removals),
                    Ok(None) => self.handle_empty(client, &mut removals),
                    Err(e) => error!("could not read from unix socket client. {e}"),
                }
            }
            self.remove_clients_by_ids(&removals);
            sleep(Duration::from_secs(1)).await;
        }
    }

    pub async fn handle(workers: Arc<Mutex<Vec<PipelineWorker>>>, rx: Receiver<UnixSocketClient>) {
        let mut handler = Self::new(workers, rx);
        handler.start().await;
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
