use crate::persist::db::schema::pipelines;
use crate::persist::db::schema::pipelines::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::Queryable;
use tracing::{debug, error};

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
    pub fn select_all(connection: &SqliteConnection) -> anyhow::Result<Vec<Self>> {
        debug!("loading all pipelines from the database");
        pipelines
            .load(connection)
            .map(|p| {
                debug!("loaded all pipelines successfully");
                p
            })
            .map_err(|e| {
                error!("could not load pipelines due to: {}", e);
                anyhow!(e)
            })
    }

    pub fn select_by_id(connection: &SqliteConnection, pip_id: &str) -> anyhow::Result<Self> {
        debug!("loading pipeline with id: {} from the database", pip_id);
        pipelines
            .filter(id.eq(pip_id))
            .first(connection)
            .map(|p| {
                debug!("loaded pipeline successfully");
                p
            })
            .map_err(|e| {
                error!("could not load pipeline due to: {}", e);
                anyhow!(e)
            })
    }

    pub fn select_by_name(connection: &SqliteConnection, pip_name: &str) -> anyhow::Result<Self> {
        debug!("loading pipeline with name: {} from the database", pip_name);
        pipelines
            .filter(name.eq(pip_name))
            .first(connection)
            .map(|p| {
                debug!("loaded pipeline successfully");
                p
            })
            .map_err(|e| {
                error!("could not load pipeline due to: {}", e);
                anyhow!(e)
            })
    }

    pub fn select_last(connection: &SqliteConnection) -> anyhow::Result<Self> {
        debug!("loading the last invoked pipeline from the database");
        pipelines
            .order(start_date_time)
            .limit(1)
            .first(connection)
            .map(|p| {
                debug!("loaded pipeline successfully");
                p
            })
            .map_err(|e| {
                error!("could not load pipeline due to: {}", e);
                anyhow!(e)
            })
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
        connection.transaction(|| {
            diesel::insert_into(pipelines::table)
                .values(&pipeline)
                .execute(connection)
                .map_err(|e| {
                    error!("could not insert pipeline due to: {}", e);
                    anyhow!(e)
                })
                .and_then(|_| {
                    debug!(
                        "created new pipeline entry for id: {}, name: {}, user: {}",
                        pip_id, pip_name, pip_user
                    );
                    Self::select_by_id(connection, pip_id)
                })
        })
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
        connection.transaction(|| {
            diesel::update(pipelines.filter(id.eq(pip_id)))
                .set((running.eq(pip_running), end_date_time.eq(pip_end_date_time)))
                .execute(connection)
                .map(|_| debug!("updated pipeline successfully"))
                .map_err(|e| {
                    error!("could not update pipeline due to: {}", e);
                    anyhow!(e)
                })
        })
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
