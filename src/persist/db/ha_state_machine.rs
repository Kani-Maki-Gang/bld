use crate::persist::db::schema::ha_state_machine;
use crate::persist::db::schema::ha_state_machine::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize, Identifiable, Insertable, Queryable)]
#[table_name = "ha_state_machine"]
pub struct HighAvailStateMachine {
    pub id: i32,
    pub last_applied_log: i32,
    pub date_created: String,
    pub date_updated: String,
}

pub fn select_first(conn: &SqliteConnection) -> anyhow::Result<HighAvailStateMachine> {
    debug!("loading the first entry of high availability state machine");
    ha_state_machine
        .first(conn)
        .map(|sm| {
            debug!("loaded high availability state machine successfully");
            sm
        })
        .map_err(|e| {
            error!(
                "could not load high availability state machine due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn select_last(conn: &SqliteConnection) -> anyhow::Result<HighAvailStateMachine> {
    debug!("loading the last entry of high availability state machine");
    ha_state_machine
        .order(id.desc())
        .first(conn)
        .map(|sm| {
            debug!("loaded high availability state machine successfully");
            sm
        })
        .map_err(|e| {
            error!(
                "could not load high availability state machine due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &SqliteConnection, sm_id: i32) -> anyhow::Result<HighAvailStateMachine> {
    debug!("loading high availability state machine with id: {}", sm_id);
    ha_state_machine
        .filter(id.eq(sm_id))
        .first(conn)
        .map(|sm| {
            debug!("loaded high availability state machine successfully");
            sm
        })
        .map_err(|e| {
            error!(
                "could not load high availability state machine due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn insert(
    conn: &SqliteConnection,
    sm_last_applied_log: i32,
) -> anyhow::Result<HighAvailStateMachine> {
    debug!(
        "inserting high availability state machine with last applied log: {:?}",
        sm_last_applied_log
    );
    conn.transaction(|| {
        diesel::insert_into(ha_state_machine)
            .values(last_applied_log.eq(sm_last_applied_log))
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not insert high availability state machine due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted high availability state machine successfully");
                select_last(conn)
            })
    })
}

pub fn update(
    conn: &SqliteConnection,
    sm_id: i32,
    sm_last_applied_log: i32,
) -> anyhow::Result<HighAvailStateMachine> {
    debug!(
        "updating high availability state machine with id: {}",
        sm_id
    );
    conn.transaction(|| {
        diesel::update(ha_state_machine.filter(id.eq(sm_id)))
            .set(last_applied_log.eq(sm_last_applied_log))
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not update high availability state machine due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated high availability state machine successfully");
                select_by_id(conn, sm_id)
            })
    })
}
