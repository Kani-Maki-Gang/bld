use crate::persist::db::schema::pipelines;
use crate::persist::db::schema::pipelines::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::Queryable;
use tracing::debug;

#[derive(Debug, Queryable)]
pub struct PipelineModel {
    pub id: String,
    pub name: String,
    pub running: bool,
    pub user: String,
    pub start_date_time: String,
    pub end_date_time: String,
}

#[derive(Insertable)]
#[table_name="pipelines"]
struct InsertPipelineModel<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub running: bool,
    pub user: &'a str,
    pub start_date_time: &'a str,
    pub end_date_time: &'a str,
}

impl PipelineModel {
    pub fn select_all(connection: &SqliteConnection) -> Option<Vec<Self>> {
        debug!("loading all pipelines from the database");
        pipelines
            .load::<Self>(connection)
            .map(|p| Some(p))
            .unwrap_or_else(|e| {
                debug!("could not load pipelines due to: {}", e);
                None
            })
    }

    pub fn select_by_id(connection: &SqliteConnection, pip_id: &str) -> Option<Self> {
        debug!("loading pipeline with id: {} from the database", pip_id);
        let result = pipelines
            .filter(id.eq(pip_id))
            .load::<Self>(connection);
        let pipeline = result
            .map(|mut p| p.pop())
            .unwrap_or_else(|e| {
                debug!("could not load pipeline due to: {}", e);
                None
            });
        if let Some(pip) = pipeline.as_ref() {
            debug!("loaded pipeline with id: {}, name: {}", pip.id, pip.name);
        }
        pipeline
    }

    pub fn select_by_name(connection: &SqliteConnection, pip_name: &str) -> Option<Self> {
        debug!("loading pipeline with name: {} from the database", pip_name);
        let result = pipelines
            .filter(name.eq(pip_name))
            .load::<Self>(connection);
        let pipeline = result
            .map(|mut p| p.pop())
            .unwrap_or_else(|e| {
                debug!("could not load pipeline due to: {}", e);
                None
            });
        if let Some(pip) = pipeline.as_ref() {
            debug!("loaded pipeline with id: {}, name: {}", pip.id, pip.name);
        }
        pipeline
    }

    pub fn select_last(connection: &SqliteConnection) -> Option<Self> {
        debug!("loading the last invoked pipeline from the database");
        let result = pipelines
            .order_by(start_date_time)
            .limit(1)
            .load::<Self>(connection);
        let pipeline = result
            .map(|mut p| p.pop())
            .unwrap_or_else(|e| {
                debug!("could not load pipeline due to: {}", e);
                None
            });
        if let Some(pip) = pipeline.as_ref() {
            debug!("loaded pipeline with id: {}, name: {}", pip.id, pip.name);
        }
        pipeline
    }

    pub fn insert(connection: &SqliteConnection, pip_id: &str, pip_name: &str, pip_user: &str) -> anyhow::Result<PipelineModel> {
        debug!("inserting new pipeline to the database");
        let pipeline = InsertPipelineModel {
            id: pip_id,
            name: pip_name,
            running: false,
            user: pip_user,
            start_date_time: &chrono::Utc::now().to_string(),
            end_date_time: "",
        };
        diesel::insert_into(pipelines::table)
            .values(&pipeline)
            .execute(connection)
            .map(|_| {
                debug!(
                    "created new pipeline entry for id: {}, name: {}, user: {}",
                    pip_id, pip_name, pip_user
                );
                Self::select_by_id(connection, pip_id).unwrap()
            })
            .map_err(|e| anyhow!(e))
    }

    pub fn update(
        connection: &SqliteConnection,
        pip_id: &str,
        pip_running: bool,
        pip_end_date_time: &str,
    ) -> anyhow::Result<()> {
        debug!(
            "updating pipeline id: {} with values running: {}, end_date_time: {}", 
            pip_id, pip_running, pip_end_date_time);
        diesel::update(pipelines.filter(id.eq(pip_id)))
            .set((running.eq(pip_running), end_date_time.eq(pip_end_date_time)))
            .execute(connection)
            .map(|_| ())
            .map_err(|e| anyhow!(e))
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
