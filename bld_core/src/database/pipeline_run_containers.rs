use crate::database::pipeline_runs::PipelineRuns;
use crate::database::schema::pipeline_run_containers;
use crate::database::schema::pipeline_run_containers::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use tracing::{debug, error};

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

pub fn select(
    conn: &SqliteConnection,
    run: &PipelineRuns,
) -> anyhow::Result<Vec<PipelineRunContainers>> {
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

pub fn select_by_id(
    conn: &SqliteConnection,
    prc_id: &str,
) -> anyhow::Result<PipelineRunContainers> {
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

pub fn insert(
    conn: &SqliteConnection,
    model: InsertPipelineRunContainer,
) -> anyhow::Result<PipelineRunContainers> {
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
) -> anyhow::Result<PipelineRunContainers> {
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
