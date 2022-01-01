use crate::persist::ha_state_machine::HighAvailStateMachine;
use crate::persist::db::schema::ha_client_status;
use crate::persist::db::schema::ha_client_status::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable};
use tracing::debug;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Identifiable, Queryable, Associations)]
#[belongs_to(HighAvailStateMachine, foreign_key = "state_machine_id")]
#[table_name = "ha_client_status"]
pub struct HighAvailClientStatus {
    pub id: i32,
    pub state_machine_id: i32,
    pub status: String,
    pub date_created: String,
    pub date_updated: String,
}

pub fn select(conn: &SqliteConnection, sm: &HighAvailStateMachine) -> anyhow::Result<Vec<HighAvailClientStatus>> {
    debug!("loading high availability client status model for state machine: {}", sm.id);
    HighAvailClientStatus::belonging_to(sm)
        .load(conn)
        .map(|cs| {
            debug!("loaded client status entries successfully");
            cs 
        })
        .map_err(|e| {
            debug!("could not load client status entries due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_last(conn: &SqliteConnection) -> anyhow::Result<HighAvailClientStatus> {
    debug!("loading last entry of high availability client status");
    ha_client_status
        .order(id.desc())
        .first(conn)
        .map(|cs| {
            debug!("loaded high availability client status successfully");
            cs
        })
        .map_err(|e| {
            debug!("could not load high availability client status due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &SqliteConnection, cs_id: i32) -> anyhow::Result<HighAvailClientStatus> {
    debug!("loading high availability client status model with id: {}", cs_id);
    ha_client_status
        .filter(id.eq(cs_id))
        .first(conn)
        .map(|cs| {
            debug!("loaded high availability client status successfully");
            cs
        })
        .map_err(|e| {
            debug!("could not load high availability client status due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert(conn: &SqliteConnection, csr_sm_id: i32, csr_status: &str) -> anyhow::Result<HighAvailClientStatus> {
    debug!("inserting high availability client status with status: {} for state machine: {}", csr_status, csr_sm_id);
    conn.transaction(|| {
        diesel::insert_into(ha_client_status)
            .values((state_machine_id.eq(csr_sm_id), status.eq(csr_status)))
            .execute(conn)
            .map_err(|e| {
                debug!("could not insert high availability client status due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted high availability client status successfully");
                select_last(conn)
            })
    })
}

pub fn update(conn: &SqliteConnection, cs_id: i32, cs_status: &str) -> anyhow::Result<HighAvailClientStatus> {
    debug!("updating high availability client status: {} with status: {}", cs_id, cs_status);
    conn.transaction(|| {
        diesel::update(ha_client_status.filter(id.eq(cs_id)))
            .set(status.eq(cs_status))
            .execute(conn)
            .map_err(|e| {
                debug!("could not update high availability client status due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated high availability client status successfully");
                select_by_id(conn, cs_id)
            })
    })
}
