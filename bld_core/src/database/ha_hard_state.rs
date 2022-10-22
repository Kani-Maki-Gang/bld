use crate::database::schema::ha_hard_state;
use crate::database::schema::ha_hard_state::dsl::*;
use anyhow::{anyhow, Result};
use async_raft::storage::HardState;
use async_raft::NodeId;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Insertable, Queryable};
use tracing::{debug, error};

#[derive(Debug, Queryable)]
pub struct HighAvailHardState {
    pub id: i32,
    pub current_term: i32,
    pub voted_for: Option<i32>,
    pub date_created: String,
    pub date_updated: String,
}

impl From<HighAvailHardState> for HardState {
    fn from(hs: HighAvailHardState) -> Self {
        Self {
            current_term: hs.current_term as u64,
            voted_for: hs.voted_for.map(|v| v as NodeId),
        }
    }
}

#[derive(Debug, Insertable)]
#[table_name = "ha_hard_state"]
pub struct InsertHighAvailHardState {
    pub current_term: i32,
    pub voted_for: Option<i32>,
}

impl InsertHighAvailHardState {
    pub fn new(hs_current_term: i32, hs_voted_for: Option<i32>) -> Self {
        Self {
            current_term: hs_current_term,
            voted_for: hs_voted_for,
        }
    }

    fn from_hard_state(hs: &HardState) -> Self {
        Self {
            current_term: hs.current_term as i32,
            voted_for: hs.voted_for.map(|v| v as i32),
        }
    }
}

impl From<HardState> for InsertHighAvailHardState {
    fn from(hs: HardState) -> Self {
        Self::from_hard_state(&hs)
    }
}

impl From<&HardState> for InsertHighAvailHardState {
    fn from(hs: &HardState) -> Self {
        Self::from_hard_state(hs)
    }
}

impl From<InsertHighAvailHardState> for HardState {
    fn from(hs: InsertHighAvailHardState) -> Self {
        Self {
            current_term: hs.current_term as u64,
            voted_for: hs.voted_for.map(|v| v as NodeId),
        }
    }
}

pub fn select_first(conn: &mut SqliteConnection) -> Result<HighAvailHardState> {
    debug!("loading the first high availability hard state");
    ha_hard_state
        .first(conn)
        .map(|ha| {
            debug!("loaded the first high availability hard state successfully");
            ha
        })
        .map_err(|e| {
            error!(
                "could not load the first high availability hard state due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn select_last(conn: &mut SqliteConnection) -> Result<HighAvailHardState> {
    debug!("loading the last entry of high availability hard state");
    ha_hard_state
        .order(id.desc())
        .first(conn)
        .map(|hs| {
            debug!("loaded high availability hard state successfully");
            hs
        })
        .map_err(|e| {
            error!("could not load high availabiilty hard state due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &mut SqliteConnection, hs_id: i32) -> Result<HighAvailHardState> {
    debug!("loading high availability hard state with id: {}", hs_id);
    ha_hard_state
        .filter(id.eq(hs_id))
        .first(conn)
        .map(|hs| {
            debug!("loaded high availability hard state successfully");
            hs
        })
        .map_err(|e| {
            error!("could not load high availability hard due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert(
    conn: &mut SqliteConnection,
    model: InsertHighAvailHardState,
) -> Result<HighAvailHardState> {
    debug!("inserting new high availability hard state: {:?}", model);
    conn.transaction(|conn| {
        diesel::insert_into(ha_hard_state::table)
            .values(model)
            .execute(conn)
            .map_err(|e| {
                error!(
                    "insert failed for high availability hard state due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("high avail hard state inserted successfully");
                select_last(conn)
            })
    })
}

pub fn update(
    conn: &mut SqliteConnection,
    hs_id: i32,
    hs_current_term: i32,
    hs_voted_for: Option<i32>,
) -> Result<HighAvailHardState> {
    debug!(
        "updateing the high availability hard state with id: {}",
        hs_id
    );
    conn.transaction(|conn| {
        diesel::update(ha_hard_state.filter(id.eq(hs_id)))
            .set((current_term.eq(hs_current_term), voted_for.eq(hs_voted_for)))
            .execute(conn)
            .map_err(|e| {
                error!(
                    "update of high availability hard state failed due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("updated the high availability hard state successfully");
                select_by_id(conn, hs_id)
            })
    })
}
