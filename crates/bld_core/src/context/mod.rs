use crate::database::pipeline_run_containers::{
    self, InsertPipelineRunContainer, PipelineRunContainers, PRC_STATE_FAULTED,
    PRC_STATE_KEEP_ALIVE, PRC_STATE_REMOVED,
};
use actix_web::rt::spawn;
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing::error;
use uuid::Uuid;

struct ContextReceiver {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    run_id: String,
    instances: Vec<PipelineRunContainers>,
}

impl ContextReceiver {
    pub fn containers_atom(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: &str,
    ) -> Self {
        Self {
            pool,
            run_id: run_id.to_string(),
            instances: vec![],
        }
    }

    pub async fn receive(mut self, mut rx: Receiver<ContextMessage>) -> Result<()> {
        while let Some(message) = rx.recv().await {
            match message {
                ContextMessage::Add(id) => self.add(&id)?,
                ContextMessage::SetAsRemoved(id) => self.set_as_removed(&id)?,
                ContextMessage::SetAsFaulted(id) => self.set_as_faulted(&id)?,
                ContextMessage::KeepAlive(id) => self.keep_alive(&id)?,
            }
        }
        Ok(())
    }

    pub fn add(&mut self, container_id: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let instance = pipeline_run_containers::insert(
            &mut conn,
            InsertPipelineRunContainer {
                id: &Uuid::new_v4().to_string(),
                run_id: &self.run_id,
                container_id,
                state: "active",
            },
        )?;
        self.instances.push(instance);
        Ok(())
    }

    pub fn set_as_removed(&mut self, container_id: &str) -> Result<()> {
        if let Some(idx) = self
            .instances
            .iter()
            .position(|i| i.container_id == container_id)
        {
            let mut conn = self.pool.get()?;
            self.instances[idx] = pipeline_run_containers::update_state(
                &mut conn,
                &self.instances[idx].id,
                PRC_STATE_REMOVED,
            )?;
        }
        Ok(())
    }

    pub fn set_as_faulted(&mut self, container_id: &str) -> Result<()> {
        if let Some(idx) = self
            .instances
            .iter()
            .position(|i| i.container_id == container_id)
        {
            let mut conn = self.pool.get()?;
            self.instances[idx] = pipeline_run_containers::update_state(
                &mut conn,
                &self.instances[idx].id,
                PRC_STATE_FAULTED,
            )?;
        }
        Ok(())
    }

    pub fn keep_alive(&mut self, container_id: &str) -> Result<()> {
        if let Some(idx) = self
            .instances
            .iter()
            .position(|i| i.container_id == container_id)
        {
            let mut conn = self.pool.get()?;
            self.instances[idx] = pipeline_run_containers::update_state(
                &mut conn,
                &self.instances[idx].id,
                PRC_STATE_KEEP_ALIVE,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ContextMessage {
    Add(String),
    SetAsRemoved(String),
    SetAsFaulted(String),
    KeepAlive(String),
}

#[derive(Default)]
pub struct ContextSender {
    tx: Option<Sender<ContextMessage>>,
}

impl ContextSender {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, run_id: &str) -> Self {
        let (tx, rx) = channel(4096);
        let context = ContextReceiver::containers_atom(pool, run_id);

        spawn(async move {
            if let Err(e) = context.receive(rx).await {
                error!("{e}");
            }
        });

        Self { tx: Some(tx) }
    }

    pub async fn add(&self, container_id: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(ContextMessage::Add(container_id)).await?;
        }
        Ok(())
    }

    pub async fn set_as_removed(&self, container_id: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(ContextMessage::SetAsRemoved(container_id)).await?;
        }
        Ok(())
    }

    pub async fn set_as_faulted(&self, container_id: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(ContextMessage::SetAsFaulted(container_id)).await?;
        }
        Ok(())
    }

    pub async fn keep_alive(&self, container_id: String) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(ContextMessage::KeepAlive(container_id)).await?;
        }
        Ok(())
    }
}
