use crate::database::schema::pipeline_runs;
use crate::database::schema::pipeline_runs::dsl::*;
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::Queryable;
use tracing::{debug, error};

pub const PR_STATE_INITIAL: &str = "initial";
pub const PR_STATE_QUEUED: &str = "queued";
pub const PR_STATE_RUNNING: &str = "running";
pub const PR_STATE_FINISHED: &str = "finished";
pub const PR_STATE_FAULTED: &str = "faulted";

#[derive(Debug, Identifiable, Queryable)]
#[diesel(table_name = pipeline_runs)]
pub struct PipelineRuns {
    pub id: String,
    pub name: String,
    pub state: String,
    pub user: String,
    pub start_date_time: String,
    pub end_date_time: Option<String>,
    pub stopped: Option<bool>,
}

#[derive(Insertable)]
#[diesel(table_name = pipeline_runs)]
struct InsertPipelineRun<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub state: &'a str,
    pub user: &'a str,
}

pub fn select_all(conn: &mut SqliteConnection) -> Result<Vec<PipelineRuns>> {
    debug!("loading all pipeline runs from the database");
    pipeline_runs
        .order(start_date_time)
        .load(conn)
        .map(|p| {
            debug!("loaded all pipeline runs successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline runs due to: {e}");
            anyhow!(e)
        })
}

pub fn select_running_by_id(conn: &mut SqliteConnection, run_id: &str) -> Result<PipelineRuns> {
    debug!("loading pipeline run with id: {run_id} that is in a running state");
    pipeline_runs
        .filter(id.eq(run_id).and(state.eq(PR_STATE_RUNNING)))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline runs successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline run due to: {e}");
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &mut SqliteConnection, pip_id: &str) -> Result<PipelineRuns> {
    debug!("loading pipeline with id: {pip_id} from the database");
    pipeline_runs
        .filter(id.eq(pip_id))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline run due to: {e}");
            anyhow!(e)
        })
}

pub fn select_by_name(conn: &mut SqliteConnection, pip_name: &str) -> Result<PipelineRuns> {
    debug!("loading pipeline with name: {pip_name} from the database");
    pipeline_runs
        .filter(name.eq(pip_name))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline run due to: {e}");
            anyhow!(e)
        })
}

pub fn select_last(conn: &mut SqliteConnection) -> Result<PipelineRuns> {
    debug!("loading the last invoked pipeline from the database");
    pipeline_runs
        .order(start_date_time.desc())
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline run due to: {e}");
            anyhow!(e)
        })
}

pub fn insert(
    conn: &mut SqliteConnection,
    pip_id: &str,
    pip_name: &str,
    pip_user: &str,
) -> Result<PipelineRuns> {
    debug!("inserting new pipeline to the database");
    let run = InsertPipelineRun {
        id: pip_id,
        name: pip_name,
        state: PR_STATE_INITIAL,
        user: pip_user,
    };
    conn.transaction(|conn| {
        diesel::insert_into(pipeline_runs::table)
            .values(&run)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert pipeline due to: {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                debug!(
                    "created new pipeline run entry for id: {}, name: {}, user: {}",
                    pip_id, pip_name, pip_user
                );
                select_by_id(conn, pip_id)
            })
    })
}

pub fn update_state(
    conn: &mut SqliteConnection,
    pip_id: &str,
    pip_state: &str,
) -> Result<PipelineRuns> {
    debug!("updating pipeline id: {pip_id} with values state: {pip_state}");
    conn.transaction(|conn| {
        diesel::update(pipeline_runs.filter(id.eq(pip_id)))
            .set(state.eq(pip_state))
            .execute(conn)
            .map_err(|e| {
                error!("could not update pipeline run due to: {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated pipeline successfully");
                select_by_id(&mut conn, pip_id)
            })
    })
}

pub fn update_stopped(
    conn: &mut SqliteConnection,
    pip_id: &str,
    pip_stopped: bool,
) -> Result<PipelineRuns> {
    debug!("updating pipeline id: {pip_id} with values stopped: {pip_stopped}");
    conn.transaction(|conn| {
        diesel::update(pipeline_runs.filter(id.eq(pip_id)))
            .set(stopped.eq(pip_stopped))
            .execute(conn)
            .map_err(|e| {
                error!("could not update pipeline run due to: {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated pipeline successfully");
                select_by_id(conn, pip_id)
            })
    })
}
