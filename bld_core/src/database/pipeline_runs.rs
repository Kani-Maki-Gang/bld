use crate::database::schema::pipeline_runs;
use crate::database::schema::pipeline_runs::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::Queryable;
use tracing::{debug, error};

#[derive(Debug, Queryable)]
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
#[table_name = "pipeline_runs"]
struct InsertPipelineRun<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub state: &'a str,
    pub user: &'a str,
}

pub fn select_all(conn: &SqliteConnection) -> anyhow::Result<Vec<PipelineRuns>> {
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

pub fn select_by_id(conn: &SqliteConnection, pip_id: &str) -> anyhow::Result<PipelineRuns> {
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

pub fn select_by_name(conn: &SqliteConnection, pip_name: &str) -> anyhow::Result<PipelineRuns> {
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

pub fn select_last(conn: &SqliteConnection) -> anyhow::Result<PipelineRuns> {
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
    conn: &SqliteConnection,
    pip_id: &str,
    pip_name: &str,
    pip_user: &str,
) -> anyhow::Result<PipelineRuns> {
    debug!("inserting new pipeline to the database");
    let run = InsertPipelineRun {
        id: pip_id,
        name: pip_name,
        state: "initial",
        user: pip_user,
    };
    conn.transaction(|| {
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
    conn: &SqliteConnection,
    pip_id: &str,
    pip_state: &str,
) -> anyhow::Result<PipelineRuns> {
    debug!("updating pipeline id: {pip_id} with values state: {pip_state}");
    conn.transaction(|| {
        diesel::update(pipeline_runs.filter(id.eq(pip_id)))
            .set(state.eq(pip_state))
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

pub fn update_stopped(
    conn: &SqliteConnection,
    pip_id: &str,
    pip_stopped: bool,
) -> anyhow::Result<PipelineRuns> {
    debug!("updating pipeline id: {pip_id} with values stopped: {pip_stopped}");
    conn.transaction(|| {
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
