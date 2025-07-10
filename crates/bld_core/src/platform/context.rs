use std::sync::Arc;

use anyhow::{Result, bail};
use bld_models::pipeline_run_containers::{
    self, InsertPipelineRunContainer, PRC_STATE_ACTIVE, PRC_STATE_FAULTED, PRC_STATE_KEEP_ALIVE,
    PRC_STATE_REMOVED,
};
use sea_orm::DatabaseConnection;
use tracing::error;
use uuid::Uuid;

pub enum PlatformContext {
    Local,
    Server {
        conn: Arc<DatabaseConnection>,
        entity_id: Option<String>,
        run_id: String,
    },
}

impl Default for PlatformContext {
    fn default() -> Self {
        Self::Local
    }
}

impl PlatformContext {
    pub fn new(run_id: &str, conn: Option<Arc<DatabaseConnection>>) -> Self {
        match conn {
            None => Self::Local,
            Some(conn) => Self::Server {
                conn: conn.clone(),
                run_id: run_id.to_string(),
                entity_id: None,
            },
        }
    }

    async fn update_pipeline_state(&self, state: &str) -> Result<()> {
        let Self::Server {
            conn, entity_id, ..
        } = &self
        else {
            return Ok(());
        };

        let Some(id) = &entity_id else {
            bail!("No entity has been created for this container");
        };
        pipeline_run_containers::update_state(conn.as_ref(), id, state)
            .await
            .map(|_| ())
    }

    pub async fn add(&mut self, container_id: &str) -> Result<()> {
        let Self::Server {
            conn,
            entity_id,
            run_id,
        } = self
        else {
            return Ok(());
        };

        let entity = pipeline_run_containers::insert(
            conn.as_ref(),
            InsertPipelineRunContainer {
                id: Uuid::new_v4().to_string(),
                run_id: run_id.to_owned(),
                container_id: container_id.to_owned(),
                state: PRC_STATE_ACTIVE.to_owned(),
            },
        )
        .await
        .map_err(|e| {
            error!("{e}");
            e
        })?;
        *entity_id = Some(entity.id);
        Ok(())
    }

    pub async fn keep_alive(&self) -> Result<()> {
        self.update_pipeline_state(PRC_STATE_KEEP_ALIVE).await
    }

    pub async fn set_as_faulted(&self) -> Result<()> {
        self.update_pipeline_state(PRC_STATE_FAULTED).await
    }

    pub async fn set_as_removed(&self) -> Result<()> {
        self.update_pipeline_state(PRC_STATE_REMOVED).await
    }
}
