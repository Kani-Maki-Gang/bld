use crate::persist::ha_state_machine::HighAvailStateMachine;
use crate::persist::db::schema::ha_client_serial_responses;
use crate::persist::db::schema::ha_client_serial_responses::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use tracing::debug;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Identifiable, Queryable, Associations)]
#[belongs_to(HighAvailStateMachine, foreign_key = "state_machine_id")]
#[table_name = "ha_client_serial_responses"]
pub struct HighAvailClientSerialResponses {
    pub id: i32,
    pub state_machine_id: i32,
    pub serial: i32,
    pub response: Option<String>,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Insertable)]
#[table_name = "ha_client_serial_responses"]
pub struct InsertHighAvailClientSerialResponses<'a> {
    pub state_machine_id: i32,
    pub serial: i32,
    pub response: Option<&'a str>
}

impl<'a> InsertHighAvailClientSerialResponses<'a> {
    pub fn new(csr_sm_id: i32, csr_serial: i32, csr_response: Option<&'a str>) -> Self {
        Self {
            state_machine_id: csr_sm_id,
            serial: csr_serial,
            response: csr_response,
        }
    }
}

pub fn select(conn: &SqliteConnection, sm: &HighAvailStateMachine) -> anyhow::Result<Vec<HighAvailClientSerialResponses>> {
    debug!("loading high availability client serial responses for state machine: {}", sm.id);
    HighAvailClientSerialResponses::belonging_to(sm)
        .load(conn)
        .map(|sr| {
            debug!("loaded serial responses successfully");
            sr
        })
        .map_err(|e| {
            debug!("could not load client serial responses due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_last(conn: &SqliteConnection) -> anyhow::Result<HighAvailClientSerialResponses> {
    debug!("loading the last high availability client serial response");
    ha_client_serial_responses
        .order(id.desc())
        .first(conn)
        .map(|csr| {
            debug!("loaded high availability client serial response successfully");
            csr
        })
        .map_err(|e| {
            debug!("could not load high availability client serial response due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &SqliteConnection, csr_id: i32) -> anyhow::Result<HighAvailClientSerialResponses> {
    debug!("loading high availability client serial response with id: {}", csr_id);
    ha_client_serial_responses
        .filter(id.eq(csr_id))
        .first(conn)
        .map(|csr| {
            debug!("loaded high availability client serial response successfully");
            csr
        })
        .map_err(|e| {
            debug!("could not load high availability client serial response due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert(conn: &SqliteConnection, model: InsertHighAvailClientSerialResponses) -> anyhow::Result<HighAvailClientSerialResponses> {
    debug!("inserting high availability client serial responses model: {:?}", model);
    conn.transaction(|| {
        diesel::insert_into(ha_client_serial_responses)
            .values(model)
            .execute(conn)
            .map_err(|e| {
                debug!("could not insert high availability client serial response due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted high availability client serial responses successfully");
                select_last(conn)
            })
    })
}
