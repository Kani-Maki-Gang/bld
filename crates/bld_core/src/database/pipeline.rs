#![allow(dead_code)]
use crate::database::schema::pipeline::dsl::*;
use crate::database::{cron_jobs, schema::pipeline};
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::Queryable;
use tracing::{debug, error};

#[derive(Debug, Queryable)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub date_created: String,
}

#[derive(Insertable)]
#[diesel(table_name = pipeline)]
struct InsertPipeline<'a> {
    pub id: &'a str,
    pub name: &'a str,
}

pub fn select_all(conn: &mut SqliteConnection) -> Result<Vec<Pipeline>> {
    debug!("loading all pipelines from the database");
    pipeline
        .load(conn)
        .map(|p| {
            debug!("loaded all pipelines successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipelines due to {e}");
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &mut SqliteConnection, pip_id: &str) -> Result<Pipeline> {
    debug!("loading pipeline with id: {pip_id} from the database");
    pipeline
        .filter(id.eq(pip_id))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to {e}");
            anyhow!(e)
        })
}

pub fn select_by_name(conn: &mut SqliteConnection, pip_name: &str) -> Result<Pipeline> {
    debug!("loading pipeline with name: {pip_name} from the database");
    pipeline
        .filter(name.eq(pip_name))
        .first(conn)
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to {e}");
            anyhow!(e)
        })
}

pub fn update_name(conn: &mut SqliteConnection, pip_id: &str, pip_name: &str) -> Result<()> {
    debug!("updating pipeline with id: {pip_id} with new name: {pip_name}");
    diesel::update(pipeline)
        .set(name.eq(pip_name))
        .filter(id.eq(pip_id))
        .execute(conn)
        .map(|_| debug!("pipeline updated successfully"))
        .map_err(|e| {
            error!("could not update pipeline due to {e}");
            anyhow!(e)
        })
}

pub fn insert(conn: &mut SqliteConnection, pip_id: &str, pip_name: &str) -> Result<Pipeline> {
    debug!("inserting new pipeline to the database");
    let model = InsertPipeline {
        id: pip_id,
        name: pip_name,
    };
    conn.transaction(|conn| {
        diesel::insert_into(pipeline::table)
            .values(&model)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert pipeline due to: {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("created new pipeline entry with id: {pip_id}, name: {pip_name}");
                select_by_id(conn, pip_id)
            })
    })
}

pub fn delete_by_name(conn: &mut SqliteConnection, pip_name: &str) -> Result<()> {
    debug!("deleting pipeline with name: {pip_name} from the database");
    conn.transaction(|conn| {
        select_by_name(conn, pip_name)
            .and_then(|pip| cron_jobs::delete_by_pipeline(conn, &pip.id))
            .and_then(|_| {
                diesel::delete(pipeline.filter(name.eq(pip_name)))
                    .execute(conn)
                    .map_err(|e| {
                        error!("could not delete pipeline due to {e}");
                        anyhow!(e)
                    })
                    .map(|_| {
                        debug!("pipeline deleted successfully");
                    })
            })
    })
}
