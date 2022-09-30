#![allow(dead_code)]
use crate::database::schema::pipeline;
use crate::database::schema::pipeline::dsl::*;
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
#[table_name = "pipeline"]
struct InsertPipeline<'a> {
    pub id: &'a str,
    pub name: &'a str,
}

pub fn select_all(conn: &SqliteConnection) -> Result<Vec<Pipeline>> {
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

pub fn select_by_id(conn: &SqliteConnection, pip_id: &str) -> Result<Pipeline> {
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

pub fn select_by_name(conn: &SqliteConnection, pip_name: &str) -> Result<Pipeline> {
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

pub fn insert(conn: &SqliteConnection, pip_id: &str, pip_name: &str) -> Result<Pipeline> {
    debug!("inserting new pipeline to the database");
    let model = InsertPipeline {
        id: pip_id,
        name: pip_name,
    };
    conn.transaction(|| {
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

pub fn delete(conn: &SqliteConnection, pip_id: &str) -> Result<()> {
    debug!("deleting pipeline with id: {pip_id} from the database");
    conn.transaction(|| {
        diesel::delete(pipeline.filter(id.eq(pip_id)))
            .execute(conn)
            .map_err(|e| {
                error!("could not delete pipeline due to {e}");
                anyhow!(e)
            })
            .map(|_| {
                debug!("pipeline deleted successfully");
            })
    })
}

pub fn delete_by_name(conn: &SqliteConnection, pip_name: &str) -> Result<()> {
    debug!("deleting pipeline with name: {pip_name} from the database");
    conn.transaction(|| {
        diesel::delete(pipeline.filter(name.eq(pip_name)))
            .execute(conn)
            .map_err(|e| {
                error!("could not delete pipeline due to {e}");
                anyhow!(e)
            })
            .map(|_| {
                debug!("pipeline delete successfully");
            })
    })
}
