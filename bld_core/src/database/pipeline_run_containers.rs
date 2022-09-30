use crate::database::pipeline_runs::PipelineRuns;
use crate::database::schema::pipeline_run_containers;
use crate::database::schema::pipeline_run_containers::dsl::*;
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use tracing::{debug, error};

pub const PRC_STATE_ACTIVE: &str = "active";
pub const PRC_STATE_REMOVED: &str = "removed";
pub const PRC_STATE_FAULTED: &str = "faulted";

#[derive(Debug, Associations, Identifiable, Queryable)]
#[belongs_to(PipelineRuns, foreign_key = "run_id")]
#[table_name = "pipeline_run_containers"]
pub struct PipelineRunContainers {
    pub id: String,
    pub run_id: String,
    pub container_id: String,
    pub state: String,
    pub date_created: String,
}

#[derive(Debug, Insertable)]
#[table_name = "pipeline_run_containers"]
pub struct InsertPipelineRunContainer<'a> {
    pub id: &'a str,
    pub run_id: &'a str,
    pub container_id: &'a str,
    pub state: &'a str,
}

pub fn select(conn: &SqliteConnection, run: &PipelineRuns) -> Result<Vec<PipelineRunContainers>> {
    debug!(
        "loading pipeline run containers for run with id: {}",
        run.id
    );
    PipelineRunContainers::belonging_to(run)
        .load(conn)
        .map(|prc| {
            debug!("loaded pipeline run containers successfully");
            prc
        })
        .map_err(|e| {
            error!("could not load pipeline run containers. {e}");
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &SqliteConnection, prc_id: &str) -> Result<PipelineRunContainers> {
    debug!("loading pipeline run container with id: {prc_id}");
    pipeline_run_containers
        .filter(id.eq(prc_id))
        .first(conn)
        .map(|prc| {
            debug!("loaded pipeline run container successfully");
            prc
        })
        .map_err(|e| {
            error!("could not load pipeline run container. {e}");
            anyhow!(e)
        })
}

pub fn select_faulted(conn: &SqliteConnection) -> Result<Vec<PipelineRunContainers>> {
    debug!("loading all faulted pipeline run containers");
    pipeline_run_containers
        .filter(state.eq(PRC_STATE_FAULTED))
        .load(conn)
        .map(|prc| {
            debug!("loaded faulted pipeline run containers successfully");
            prc
        })
        .map_err(|e| {
            debug!("could not load pipeline run containers, {e}");
            anyhow!(e)
        })
}

pub fn insert(
    conn: &SqliteConnection,
    model: InsertPipelineRunContainer,
) -> Result<PipelineRunContainers> {
    debug!("inserting pipeline run container");
    conn.transaction(|| {
        diesel::insert_into(pipeline_run_containers)
            .values(&model)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert pipeline run container. {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted pipeline run container successfully");
                select_by_id(conn, model.id)
            })
    })
}

pub fn update_state(
    conn: &SqliteConnection,
    prc_id: &str,
    prc_state: &str,
) -> Result<PipelineRunContainers> {
    debug!("updating pipeline run container with id: {prc_id} with new state: {prc_state}");
    conn.transaction(|| {
        diesel::update(pipeline_run_containers.filter(id.eq(prc_id)))
            .set(state.eq(prc_state))
            .execute(conn)
            .map_err(|e| {
                error!("could not update pipeline run container. {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated pipeline run containers successfully");
                select_by_id(conn, prc_id)
            })
    })
}

pub fn update_running_containers_to_faulted(
    conn: &SqliteConnection,
    prc_run_id: &str,
) -> Result<()> {
    debug!(
        "updating all pipeline run containers of run id: {} from state running to faulted",
        prc_run_id
    );
    conn.transaction(|| {
        diesel::update(
            pipeline_run_containers.filter(run_id.eq(prc_run_id).and(state.eq(PRC_STATE_ACTIVE))),
        )
        .set(state.eq(PRC_STATE_FAULTED))
        .execute(conn)
        .map_err(|e| {
            error!("could not update pipeline run containers, {e}");
            anyhow!(e)
        })
        .map(|_| ())
    })
}
