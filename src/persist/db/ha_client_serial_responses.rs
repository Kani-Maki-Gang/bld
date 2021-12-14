use crate::persist::HighAvailStateMachineModel;
use crate::persist::db::schema::ha_client_serial_responses;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Identifiable, Queryable, Associations)]
#[belongs_to(HighAvailStateMachineModel, foreign_key = "state_machine_id")]
#[table_name = "ha_client_serial_responses"]
pub struct HighAvailClientSerialResponsesModel {
    pub id: String,
    pub state_machine_id: String,
    pub serial: String,
    pub previous: Option<String>,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailClientSerialResponsesModel {
    pub fn select(conn: &SqliteConnection, sm: &HighAvailStateMachineModel) -> anyhow::Result<Vec<Self>> {
        debug!("loading high availability client serial responses for state machine: {}", sm.id);
        HighAvailClientSerialResponsesModel::belonging_to(sm)
            .load::<Self>(conn)
            .map(|sr| {
                debug!("loaded serial responses successfully");
                sr
            })
            .map_err(|e| {
                debug!("could not load client serial responses due to: {}", e);
                anyhow!(e)
            })
    }
}
