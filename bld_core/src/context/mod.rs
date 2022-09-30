use crate::database::pipeline_run_containers::{
    self, InsertPipelineRunContainer, PipelineRunContainers, PRC_STATE_FAULTED, PRC_STATE_REMOVED,
};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub enum Context {
    Empty,
    Containers {
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: String,
        instances: Vec<PipelineRunContainers>,
    },
}

impl Context {
    pub fn containers_atom(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: &str,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::Containers {
            pool,
            run_id: run_id.to_string(),
            instances: vec![],
        }))
    }

    pub fn add(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Containers {
                pool,
                run_id,
                instances,
            } => {
                let conn = pool.get()?;
                let instance = pipeline_run_containers::insert(
                    &conn,
                    InsertPipelineRunContainer {
                        id: &Uuid::new_v4().to_string(),
                        run_id: &run_id,
                        container_id,
                        state: "active",
                    },
                )?;
                instances.push(instance);
                Ok(())
            }
        }
    }

    pub fn remove(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Containers {
                pool,
                run_id: _,
                instances,
            } => {
                if let Some(idx) = instances
                    .iter()
                    .position(|i| i.container_id == container_id)
                {
                    let conn = pool.get()?;
                    instances[idx] = pipeline_run_containers::update_state(
                        &conn,
                        &instances[idx].id,
                        PRC_STATE_REMOVED,
                    )?;
                }
                Ok(())
            }
        }
    }

    pub fn faulted(&mut self, container_id: &str) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Containers {
                pool,
                run_id: _,
                instances,
            } => {
                if let Some(idx) = instances
                    .iter()
                    .position(|i| i.container_id == container_id)
                {
                    let conn = pool.get()?;
                    instances[idx] = pipeline_run_containers::update_state(
                        &conn,
                        &instances[idx].id,
                        PRC_STATE_FAULTED,
                    )?;
                }
                Ok(())
            }
        }
    }
}
