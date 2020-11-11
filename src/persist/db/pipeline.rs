use crate::helpers::err;
use crate::persist::db::queries::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_types::{Bool, Text};
use diesel::sqlite::SqliteConnection;
use diesel::{sql_query, Queryable, QueryableByName};
use std::io;

#[derive(Debug, Queryable, QueryableByName)]
pub struct PipelineModel {
    #[sql_type = "Text"]
    pub id: String,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "Bool"]
    pub running: bool,
}

impl PipelineModel {
    pub fn create(connection: &SqliteConnection) -> io::Result<()> {
        if let Err(e) = sql_query(CREATE_TABLE_PIPELINE_QUERY).execute(connection) {
            return err(e.to_string());
        }
        Ok(())
    }

    pub fn select_all(connection: &SqliteConnection) -> io::Result<Vec<Self>> {
        let query = sql_query(SELECT_PIPELINES_QUERY).load::<Self>(connection);
        if let Err(e) = query {
            return err(e.to_string());
        }
        Ok(query.unwrap())
    }

    pub fn select_by_id(connection: &SqliteConnection, id: &str) -> Option<Self> {
        let query = sql_query(SELECT_PIPELINE_BY_ID_QUERY)
            .bind::<Text, _>(id)
            .load::<Self>(connection);
        if let Err(_) = query {
            return None;
        }
        query.unwrap().pop()
    }

    pub fn insert(connection: &SqliteConnection, pipeline: &Self) -> io::Result<()> {
        let query = sql_query(INSERT_PIPELINE_QUERY)
            .bind::<Text, _>(&pipeline.id)
            .bind::<Text, _>(&pipeline.name)
            .bind::<Bool, _>(pipeline.running)
            .execute(connection);
        if let Err(e) = query {
            return err(e.to_string());
        }
        Ok(())
    }

    pub fn update(connection: &SqliteConnection, id: &str, running: bool) -> io::Result<()> {
        let query = sql_query(UPDATE_PIPELINE_QUERY)
            .bind::<Bool, _>(running)
            .bind::<Text, _>(id)
            .execute(connection);
        if let Err(e) = query {
            return err(e.to_string());
        }
        Ok(())
    }
}
