use actix_web::{rt::spawn, web::Data};
use anyhow::{anyhow, Error, Result};
use bld_config::BldConfig;
use bld_core::{
    database::{
        pipeline_run_containers::{self, PRC_STATE_REMOVED},
        pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_QUEUED},
    },
    workers::PipelineWorker,
};
use sea_orm::DatabaseConnection;
use shiplift::{errors::Error as ShipliftError, Docker, RmContainerOptions};
use std::collections::VecDeque;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

fn oneshot_send_err<T>(_: T) -> Error {
    anyhow!("oneshot receiver dropped")
}

#[derive(Debug)]
pub enum WorkerQueueMessage {
    Enqueue {
        worker: PipelineWorker,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Dequeue {
        pid: u32,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Stop {
        run_id: String,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Contains {
        pid: u32,
        resp_tx: oneshot::Sender<bool>,
    },
}

/// The WorkerQueueReceiver is initialized with a capacity of active workers.
/// If there are more workers than the specified capacity, the queue manager
/// will add them to a backlog based on when they were enqueued.
struct WorkerQueueReceiver {
    capacity: usize,
    active: Vec<PipelineWorker>,
    backlog: VecDeque<PipelineWorker>,
    config: Data<BldConfig>,
    conn: Data<DatabaseConnection>,
    rx: mpsc::Receiver<WorkerQueueMessage>,
}

impl WorkerQueueReceiver {
    pub fn new(
        capacity: usize,
        config: Data<BldConfig>,
        conn: Data<DatabaseConnection>,
        rx: mpsc::Receiver<WorkerQueueMessage>,
    ) -> Self {
        let config_clone = config.clone();
        let conn_clone = conn.clone();

        spawn(async move {
            if let Err(e) = try_cleanup_containers(config_clone, conn_clone).await {
                error!("error while cleaning up containers, {e}");
            }
        });

        Self {
            capacity,
            active: Vec::with_capacity(capacity),
            backlog: VecDeque::new(),
            config,
            conn,
            rx,
        }
    }

    pub async fn receive(mut self) -> Result<()> {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                WorkerQueueMessage::Enqueue { worker, resp_tx } => {
                    let result = self.enqueue(worker).await;
                    resp_tx.send(result).map_err(oneshot_send_err)?;
                }
                WorkerQueueMessage::Dequeue { pid, resp_tx } => {
                    let result = self.dequeue(pid).await;
                    resp_tx.send(result).map_err(oneshot_send_err)?;
                }
                WorkerQueueMessage::Stop { run_id, resp_tx } => {
                    let result = self.stop(run_id).await;
                    resp_tx.send(result).map_err(oneshot_send_err)?;
                }
                WorkerQueueMessage::Contains { pid, resp_tx } => {
                    let result = self.contains(pid);
                    resp_tx.send(result).map_err(oneshot_send_err)?;
                }
            }
        }
        Ok(())
    }

    fn activate(&mut self, mut worker: PipelineWorker) -> Result<()> {
        worker.spawn().map_err(|e| {
            error!("{e}");
            e
        })?;
        self.active.push(worker);
        Ok(())
    }

    async fn add_backlog(&mut self, worker: PipelineWorker) -> Result<()> {
        pipeline_runs::update_state(self.conn.as_ref(), worker.get_run_id(), PR_STATE_QUEUED)
            .await?;
        self.backlog.push_back(worker);
        Ok(())
    }

    fn after_removal(&mut self) -> Result<()> {
        for _ in 0..(self.capacity - self.active.len()) {
            if let Some(worker) = self.backlog.pop_front() {
                self.activate(worker)?;
            }
        }

        let config = self.config.clone();
        let conn = self.conn.clone();
        spawn(async move {
            if let Err(e) = try_cleanup_containers(config, conn).await {
                error!("error while cleaning up containers, {e}");
            }
        });

        Ok(())
    }

    /// Used to spawn the child process of the worker and add it to the active workers vector.
    async fn enqueue(&mut self, item: PipelineWorker) -> Result<()> {
        if self.active.len() < self.capacity {
            self.activate(item)?;
        } else {
            self.add_backlog(item).await?;
        }
        Ok(())
    }

    /// This method will check for a worker that have finished executing and will remove them from
    /// the active workers collection. It will pop the appropriate amount of workers from the
    /// backlog vector, spawn them and add them as active.
    async fn dequeue(&mut self, pid: u32) -> Result<()> {
        let mut cleanup = vec![];
        let mut i = 0;

        while i < self.active.len() {
            if self.active[i]
                .get_pid()
                .as_ref()
                .map(|wpid| pid == *wpid)
                .unwrap_or(false)
            {
                let worker = self.active.remove(i);
                cleanup.push(worker);
            } else {
                i += 1;
            }
        }

        for entry in cleanup.iter_mut() {
            if let Err(e) = try_cleanup_process(self.conn.clone(), entry).await {
                error!("error while cleaning up worker process, {e}");
            }
        }

        self.after_removal()?;
        Ok(())
    }

    async fn stop(&mut self, run_id: String) -> Result<()> {
        let mut found_in_active = false;
        let mut stopped = vec![];
        let mut i = 0;

        while i < self.active.len() {
            if self.active[i].has_run_id(&run_id) {
                let worker = self.active.remove(i);
                found_in_active = true;
                stopped.push(worker);
            } else {
                i += 1;
            }
        }
        self.active.retain_mut(|w| {
            let found = w.has_run_id(&run_id);
            if found {}
            !found
        });

        for entry in stopped.iter_mut() {
            if let Err(e) = entry.stop() {
                error!("error while stopping worker process: {e}");
            }
            if let Err(e) = try_cleanup_process(self.conn.clone(), entry).await {
                error!("error while cleaning up worker process, {e}");
            }
        }

        if found_in_active {
            self.after_removal()?;
        } else {
            self.backlog.retain(|w| !w.has_run_id(&run_id));
        }

        Ok(())
    }

    fn contains(&mut self, pid: u32) -> bool {
        self.active
            .iter()
            .find(|w| w.has_pid(pid))
            .or_else(|| self.backlog.iter().find(|w| w.has_pid(pid)))
            .is_some()
    }
}

pub struct WorkerQueueSender {
    tx: mpsc::Sender<WorkerQueueMessage>,
}

impl WorkerQueueSender {
    pub fn new(tx: mpsc::Sender<WorkerQueueMessage>) -> Self {
        Self { tx }
    }

    pub async fn enqueue(&self, worker: PipelineWorker) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let message = WorkerQueueMessage::Enqueue { worker, resp_tx };

        self.tx.send(message).await.map_err(|e| anyhow!(e))?;

        resp_rx.await?
    }

    pub async fn dequeue(&self, pid: u32) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let message = WorkerQueueMessage::Dequeue { pid, resp_tx };

        self.tx.send(message).await.map_err(|e| anyhow!(e))?;

        resp_rx.await?
    }

    pub async fn stop(&self, run_id: &str) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let message = WorkerQueueMessage::Stop {
            run_id: run_id.to_owned(),
            resp_tx,
        };

        self.tx.send(message).await.map_err(|e| anyhow!(e))?;

        resp_rx.await?
    }

    pub async fn contains(&self, pid: u32) -> Result<bool> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let message = WorkerQueueMessage::Contains { pid, resp_tx };

        self.tx.send(message).await.map_err(|e| anyhow!(e))?;

        resp_rx.await.map_err(|e| anyhow!(e))
    }
}

pub fn worker_queue_channel(
    capacity: usize,
    config: Data<BldConfig>,
    conn: Data<DatabaseConnection>,
) -> WorkerQueueSender {
    let (tx, rx) = mpsc::channel(4096);
    let receiver = WorkerQueueReceiver::new(capacity, config, conn, rx);

    spawn(async move {
        if let Err(e) = receiver.receive().await {
            error!("{e}");
        }
    });

    WorkerQueueSender::new(tx)
}

/// This function will call the clean up method for the worker and check
/// the current state of the run id. If its set as running, the worker did not
/// complete successfully so it will be set to finished and all of its associated
/// containers will be set as faulted in order to be cleaned up later.
async fn try_cleanup_process(
    conn: Data<DatabaseConnection>,
    worker: &mut PipelineWorker,
) -> Result<()> {
    debug!("starting worker process cleanup");

    let conn = conn.as_ref();

    if let Err(e) = worker.cleanup() {
        error!("error when trying to cleanup the worker process, {e}");
    }

    let run_id = worker.get_run_id();
    let run = pipeline_runs::select_running_by_id(conn, run_id).await?;

    if run.state != PR_STATE_FINISHED || run.state != PR_STATE_FAULTED {
        let _ = pipeline_runs::update_state(conn, run_id, PR_STATE_FAULTED);
    }

    let _ = pipeline_run_containers::update_running_containers_to_faulted(conn, run_id).await;

    Ok(())
}

/// This function will fetch all containers with faulted state or those in active state
/// with runs that have either finished or faulted, and try to stop and remove them using the docker
/// engine API and then set their state as removed.
pub async fn try_cleanup_containers(
    config: Data<BldConfig>,
    conn: Data<DatabaseConnection>,
) -> Result<()> {
    let run_containers = pipeline_run_containers::select_in_invalid_state(conn.as_ref()).await?;

    info!("found {} containers in invalid state", run_containers.len());

    let url = config.local.docker_url.parse()?;
    let client = Docker::host(url);

    for info in run_containers {
        let container = client.containers().get(&info.container_id);

        let container_found = match container.stop(None).await {
            // container doesn't exist, move to the next part
            Err(ShipliftError::Fault { code, .. }) if code.as_u16() == 404 => false,
            Err(e) => {
                error!("could not stop container {}, {:?}", info.container_id, e);
                true
            }
            _ => true,
        };

        if container_found {
            let options = RmContainerOptions::builder().force(true).build();
            match container.remove(options).await {
                // container doesn't exist, move to the next part
                Err(ShipliftError::Fault { code, .. }) if code.as_u16() == 404 => {}
                Err(e) => {
                    error!("could not remove container {}, {:?}", info.container_id, e);
                    continue;
                }
                _ => {}
            }
        }

        let _ =
            pipeline_run_containers::update_state(conn.as_ref(), &info.id, PRC_STATE_REMOVED).await;
    }

    Ok(())
}
