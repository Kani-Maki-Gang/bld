use crate::database::ha_state_machine::HighAvailStateMachine;
use crate::database::schema::ha_client_serial_responses;
use crate::database::schema::ha_client_serial_responses::dsl::*;
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize, Identifiable, Queryable, Associations)]
#[diesel(belongs_to(HighAvailStateMachine, foreign_key = state_machine_id))]
#[diesel(table_name = ha_client_serial_responses)]
pub struct HighAvailClientSerialResponses {
    pub id: i32,
    pub state_machine_id: i32,
    pub serial: i32,
    pub response: Option<String>,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = ha_client_serial_responses)]
pub struct InsertHighAvailClientSerialResponses<'a> {
    pub id: i32,
    pub state_machine_id: i32,
    pub serial: i32,
    pub response: Option<&'a str>,
}

impl<'a> InsertHighAvailClientSerialResponses<'a> {
    pub fn new(
        csr_id: i32,
        csr_sm_id: i32,
        csr_serial: i32,
        csr_response: Option<&'a str>,
    ) -> Self {
        Self {
            id: csr_id,
            state_machine_id: csr_sm_id,
            serial: csr_serial,
            response: csr_response,
        }
    }
}

pub fn select_last(conn: &mut SqliteConnection) -> Result<HighAvailClientSerialResponses> {
    debug!("loading the last high availability client serial response");
    ha_client_serial_responses
        .order(id.desc())
        .first(conn)
        .map(|csr| {
            debug!("loaded high availability client serial response successfully");
            csr
        })
        .map_err(|e| {
            error!(
                "could not load high availability client serial response due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn select_by_id(
    conn: &mut SqliteConnection,
    csr_id: i32,
) -> Result<HighAvailClientSerialResponses> {
    debug!(
        "loading high availability client serial response with id: {}",
        csr_id
    );
    ha_client_serial_responses
        .filter(id.eq(csr_id))
        .first(conn)
        .map(|csr| {
            debug!("loaded high availability client serial response successfully");
            csr
        })
        .map_err(|e| {
            error!(
                "could not load high availability client serial response due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn insert(
    conn: &mut SqliteConnection,
    model: InsertHighAvailClientSerialResponses,
) -> Result<HighAvailClientSerialResponses> {
    debug!(
        "inserting high availability client serial responses model: {:?}",
        model
    );
    conn.transaction(|conn| {
        diesel::insert_into(ha_client_serial_responses)
            .values(model)
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not insert high availability client serial response due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted high availability client serial responses successfully");
                select_last(conn)
            })
    })
}
