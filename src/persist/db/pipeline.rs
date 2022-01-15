use crate::persist::db::schema::pipelines;
use crate::persist::db::schema::pipelines::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::Queryable;
use tracing::{debug, error};

#[derive(Debug, Queryable)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub running: bool,
    pub user: String,
    pub start_date_time: String,
    pub end_date_time: Option<String>,
}

#[derive(Insertable)]
#[table_name = "pipelines"]
struct InsertPipeline<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub running: bool,
    pub user: &'a str,
}

pub fn select_all(conn: &SqliteConnection) -> anyhow::Result<Vec<Pipeline>> {
    debug!("loading all pipelines from the database");
    pipelines
        .order(start_date_time)
        .load(conn)
        .map(|p| {
            debug!("loaded all pipelines successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipelines due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &SqliteConnection, pip_id: &str) -> anyhow::Result<Pipeline> {
    debug!("loading pipeline with id: {} from the database", pip_id);
    pipelines
        .filter(id.eq(pip_id))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_name(conn: &SqliteConnection, pip_name: &str) -> anyhow::Result<Pipeline> {
    debug!("loading pipeline with name: {} from the database", pip_name);
    pipelines
        .filter(name.eq(pip_name))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_last(conn: &SqliteConnection) -> anyhow::Result<Pipeline> {
    debug!("loading the last invoked pipeline from the database");
    pipelines
        .order(start_date_time)
        .limit(1)
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert(
    conn: &SqliteConnection,
    pip_id: &str,
    pip_name: &str,
    pip_user: &str,
) -> anyhow::Result<Pipeline> {
    debug!("inserting new pipeline to the database");
    let pipeline = InsertPipeline {
        id: pip_id,
        name: pip_name,
        running: false,
        user: pip_user,
    };
    conn.transaction(|| {
        diesel::insert_into(pipelines::table)
            .values(&pipeline)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert pipeline due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!(
                    "created new pipeline entry for id: {}, name: {}, user: {}",
                    pip_id, pip_name, pip_user
                );
                select_by_id(conn, pip_id)
            })
    })
}

pub fn update(
    conn: &SqliteConnection,
    pip_id: &str,
    pip_running: bool,
) -> anyhow::Result<Pipeline> {
    debug!(
        "updating pipeline id: {} with values running: {}",
        pip_id, pip_running
    );
    conn.transaction(|| {
        diesel::update(pipelines.filter(id.eq(pip_id)))
            .set(running.eq(pip_running))
            .execute(conn)
            .map_err(|e| {
                error!("could not update pipeline due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated pipeline successfully");
                select_by_id(conn, pip_id)
            })
    })
}
