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
    #[sql_type = "Text"]
    pub user: String,
    #[sql_type = "Text"]
    pub start_date_time: String,
    #[sql_type = "Text"]
    pub end_date_time: String,
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
        if query.is_err() {
            return None;
        }
        query.unwrap().pop()
    }

    pub fn select_by_name(connection: &SqliteConnection, name: &str) -> Option<Self> {
        let query = sql_query(SELECT_PIPELINE_BY_NAME_QUERY)
            .bind::<Text, _>(name)
            .load::<Self>(connection);
        if query.is_err() {
            return None;
        }
        query.unwrap().pop()
    }

    pub fn select_last(connection: &SqliteConnection) -> Option<Self> {
        let query = sql_query(SELECT_LAST_INVOKED_PIPELINE).load::<Self>(connection);
        if query.is_err() {
            return None;
        }
        query.unwrap().pop()
    }

    pub fn insert(connection: &SqliteConnection, pipeline: &Self) -> Result<()> {
        sql_query(INSERT_PIPELINE_QUERY)
            .bind::<Text, _>(&pipeline.id)
            .bind::<Text, _>(&pipeline.name)
            .bind::<Bool, _>(pipeline.running)
            .bind::<Text, _>(&pipeline.user)
            .bind::<Text, _>(&pipeline.start_date_time)
            .bind::<Text, _>(&pipeline.end_date_time)
            .execute(connection)?;
        Ok(())
    }

    pub fn update(
        connection: &SqliteConnection,
        id: &str,
        running: bool,
        end_date_time: &str,
    ) -> Result<()> {
        sql_query(UPDATE_PIPELINE_QUERY)
            .bind::<Bool, _>(running)
            .bind::<Text, _>(end_date_time)
            .bind::<Text, _>(id)
            .execute(connection)?;
        Ok(())
    }
}

impl ToString for PipelineModel {
    fn to_string(&self) -> String {
        let mut info = String::new();
        info.push_str(&format!("ID: {}\n", self.id));
        info.push_str(&format!("NAME: {}\n", self.name));
        info.push_str(&format!("USER: {}\n", self.user));
        info.push_str(&format!("IS RUNNING: {}\n", self.running));
        info.push_str(&format!("START TIME: {}\n", self.start_date_time));
        info.push_str(&format!("END TIME: {}", self.end_date_time));
        info
    }
}
