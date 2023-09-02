use crate::database::{
    pipeline_runs::{PipelineRuns, PR_STATE_FAULTED, PR_STATE_FINISHED},
    schema::{
        pipeline_run_containers, pipeline_run_containers::dsl::*, pipeline_runs::dsl as pr_dsl,
    },
};
use anyhow::{anyhow, Result};
use diesel::{
    prelude::*,
    query_dsl::{QueryDsl, RunQueryDsl},
    Associations, Identifiable, Queryable,
};
use tracing::{debug, error};

use super::DbConnection;

pub const PRC_STATE_ACTIVE: &str = "active";
pub const PRC_STATE_REMOVED: &str = "removed";
pub const PRC_STATE_FAULTED: &str = "faulted";
pub const PRC_STATE_KEEP_ALIVE: &str = "keep-alive"; // Set when the pipeline is configured to not dispose.

#[derive(Clone, Debug, Associations, Identifiable, Queryable)]
#[diesel(belongs_to(PipelineRuns, foreign_key = run_id))]
#[diesel(table_name = pipeline_run_containers)]
pub struct PipelineRunContainers {
    pub id: String,
    pub run_id: String,
    pub container_id: String,
    pub state: String,
    pub date_created: String,
}

#[derive(Debug)]
pub struct InsertPipelineRunContainer<'a> {
    pub id: &'a str,
    pub run_id: &'a str,
    pub container_id: &'a str,
    pub state: &'a str,
}

pub fn select(conn: &mut DbConnection, run: &PipelineRuns) -> Result<Vec<PipelineRunContainers>> {
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

pub fn select_by_id(conn: &mut DbConnection, prc_id: &str) -> Result<PipelineRunContainers> {
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

pub fn select_in_invalid_state(conn: &mut DbConnection) -> Result<Vec<PipelineRunContainers>> {
    debug!("loading all pipeline run containers that are in an invalid state");
    let active_containers: Vec<(PipelineRuns, PipelineRunContainers)> = pr_dsl::pipeline_runs
        .inner_join(pipeline_run_containers)
        .filter(
            pr_dsl::state
                .eq_any(&[PR_STATE_FINISHED, PR_STATE_FAULTED])
                .and(state.eq(PRC_STATE_ACTIVE)),
        )
        .load(conn)
        .map(|res| {
            debug!(
                "loaded active pipeline run containers with finished or faulted runs, successfully"
            );
            res
        })
        .map_err(|e| {
            error!("could not load pipeline run containers, {e}");
            anyhow!(e)
        })?;
    let mut faulted_containers = pipeline_run_containers
        .filter(state.eq(PRC_STATE_FAULTED))
        .load(conn)
        .map(|prc| {
            debug!("loaded faulted pipeline run containers successfully");
            prc
        })
        .map_err(|e| {
            error!("could not load pipeline run containers, {e}");
            anyhow!(e)
        })?;
    faulted_containers.append(&mut active_containers.into_iter().map(|r| r.1).collect());
    Ok(faulted_containers)
}

pub fn insert(
    conn: &mut DbConnection,
    model: InsertPipelineRunContainer,
) -> Result<PipelineRunContainers> {
    debug!("inserting pipeline run container");
    conn.transaction(|conn| {
        diesel::insert_into(pipeline_run_containers)
            .values((
                id.eq(model.id),
                run_id.eq(model.run_id),
                container_id.eq(model.container_id),
                state.eq(model.state),
            ))
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
    conn: &mut DbConnection,
    prc_id: &str,
    prc_state: &str,
) -> Result<PipelineRunContainers> {
    debug!("updating pipeline run container with id: {prc_id} with new state: {prc_state}");
    conn.transaction(|conn| {
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
    conn: &mut DbConnection,
    prc_run_id: &str,
) -> Result<()> {
    debug!(
        "updating all pipeline run containers of run id: {} from state running to faulted",
        prc_run_id
    );
    conn.transaction(|conn| {
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
