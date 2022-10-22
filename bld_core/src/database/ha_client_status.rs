use crate::database::ha_state_machine::HighAvailStateMachine;
use crate::database::schema::ha_client_status;
use crate::database::schema::ha_client_status::dsl::*;
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

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

#[derive(Debug, Insertable)]
#[table_name = "ha_client_status"]
pub struct InsertHighAvailClientStatus<'a> {
    pub id: i32,
    pub state_machine_id: i32,
    pub status: &'a str,
}

impl<'a> InsertHighAvailClientStatus<'a> {
    pub fn new(cs_id: i32, cs_state_machine_id: i32, cs_status: &'a str) -> Self {
        Self {
            id: cs_id,
            state_machine_id: cs_state_machine_id,
            status: cs_status,
        }
    }
}

pub fn select_last(conn: &mut SqliteConnection) -> Result<HighAvailClientStatus> {
    debug!("loading last entry of high availability client status");
    ha_client_status
        .order(id.desc())
        .first(conn)
        .map(|cs| {
            debug!("loaded high availability client status successfully");
            cs
        })
        .map_err(|e| {
            error!(
                "could not load high availability client status due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &mut SqliteConnection, cs_id: i32) -> Result<HighAvailClientStatus> {
    debug!(
        "loading high availability client status model with id: {}",
        cs_id
    );
    ha_client_status
        .filter(id.eq(cs_id))
        .first(conn)
        .map(|cs| {
            debug!("loaded high availability client status successfully");
            cs
        })
        .map_err(|e| {
            error!(
                "could not load high availability client status due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn insert(
    conn: &mut SqliteConnection,
    model: InsertHighAvailClientStatus,
) -> Result<HighAvailClientStatus> {
    debug!(
        "inserting high availability client status with status: {} for state machine: {}",
        model.status, model.state_machine_id
    );
    conn.transaction(|| {
        diesel::insert_into(ha_client_status)
            .values(model)
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not insert high availability client status due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted high availability client status successfully");
                select_last(conn)
            })
    })
}

pub fn update(
    conn: &mut SqliteConnection,
    cs_id: i32,
    cs_status: &str,
) -> Result<HighAvailClientStatus> {
    debug!(
        "updating high availability client status: {} with status: {}",
        cs_id, cs_status
    );
    conn.transaction(|| {
        diesel::update(ha_client_status.filter(id.eq(cs_id)))
            .set(status.eq(cs_status))
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not update high availability client status due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated high availability client status successfully");
                select_by_id(conn, cs_id)
            })
    })
}
