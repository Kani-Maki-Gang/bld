use crate::persist::db::schema::ha_state_machine;
use crate::persist::db::schema::ha_state_machine::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Identifiable, Queryable)]
#[table_name = "ha_state_machine"]
pub struct HighAvailStateMachineModel {
    pub id: String,
    pub last_applied_log: f64,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailStateMachineModel {
    pub fn select(conn: &SqliteConnection, sm_id: &str) -> anyhow::Result<Self> {
        debug!("loading high availability state machine with id: {}", sm_id);
        ha_state_machine
            .filter(id.eq(sm_id))
            .first::<Self>(conn)
            .map_err(|e| {
                debug!("could not load high availability state machine due to: {}", e);
                anyhow!(e)
            })
    }
}
