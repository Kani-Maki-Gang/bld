use crate::persist::HighAvailStateMachineModel;
use crate::persist::db::schema::ha_client_status;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Identifiable, Queryable, Associations)]
#[belongs_to(HighAvailStateMachineModel, foreign_key = "state_machine_id")]
#[table_name = "ha_client_status"]
pub struct HighAvailClientStatusModel {
    pub id: String,
    pub state_machine_id: String,
    pub status: Option<String>,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailClientStatusModel {
    pub fn select(conn: &SqliteConnection, sm: &HighAvailStateMachineModel) -> anyhow::Result<Vec<Self>> {
        debug!("loading high availability client status model for state machine: {}", sm.id);
        HighAvailClientStatusModel::belonging_to(sm)
            .load::<Self>(conn)
            .map(|sr| {
                debug!("loaded client status entries successfully");
                sr
            })
            .map_err(|e| {
                debug!("could not load client status entries due to: {}", e);
                anyhow!(e)
            })
    }
}
