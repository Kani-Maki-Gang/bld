use crate::persist::db::queries::*;
use crate::types::Result;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_types::{Bool, Text};
use diesel::sqlite::SqliteConnection;
use diesel::{sql_query, Queryable, QueryableByName};

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
    pub fn create(connection: &SqliteConnection) -> Result<()> {
        sql_query(CREATE_TABLE_PIPELINE_QUERY).execute(connection)?;
        Ok(())
    }

    pub fn select_all(connection: &SqliteConnection) -> Result<Vec<Self>> {
        let res = sql_query(SELECT_PIPELINES_QUERY).load::<Self>(connection)?;
        Ok(res)
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

    pub fn insert(connection: &SqliteConnection, pipeline: &Self) -> Result<()> {
        sql_query(INSERT_PIPELINE_QUERY)
            .bind::<Text, _>(&pipeline.id)
            .bind::<Text, _>(&pipeline.name)
            .bind::<Bool, _>(pipeline.running)
            .execute(connection)?;
        Ok(())
    }

    pub fn update(connection: &SqliteConnection, id: &str, running: bool) -> Result<()> {
        sql_query(UPDATE_PIPELINE_QUERY)
            .bind::<Bool, _>(running)
            .bind::<Text, _>(id)
            .execute(connection)?;
        Ok(())
    }
}
