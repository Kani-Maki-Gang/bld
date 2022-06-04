use crate::database::schema::ha_snapshot;
use crate::database::schema::ha_snapshot::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize, Identifiable, Queryable)]
#[table_name = "ha_snapshot"]
pub struct HighAvailSnapshot {
    pub id: i32,
    pub term: i32,
    pub data: Vec<u8>,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Insertable)]
#[table_name = "ha_snapshot"]
pub struct InsertHighAvailSnapshot {
    pub id: i32,
    pub term: i32,
    pub data: Vec<u8>,
}

impl InsertHighAvailSnapshot {
    pub fn new(sn_id: i32, sn_term: i32, sn_data: Vec<u8>) -> Self {
        Self {
            id: sn_id,
            term: sn_term,
            data: sn_data,
        }
    }
}

pub fn select_last(conn: &SqliteConnection) -> anyhow::Result<HighAvailSnapshot> {
    debug!("loading the last entry high availability snapshot");
    ha_snapshot
        .order(id.desc())
        .first(conn)
        .map(|sn| {
            debug!("loaded high availability snapshot successfully");
            sn
        })
        .map_err(|e| {
            error!("could not load high availability snapshot due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert(
    conn: &SqliteConnection,
    model: InsertHighAvailSnapshot,
) -> anyhow::Result<HighAvailSnapshot> {
    debug!("inserting high availability snapshot: {:?}", model);
    conn.transaction(|| {
        diesel::insert_into(ha_snapshot)
            .values(&model)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert high availability snapshot due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("inserted high availability snapshot successfully");
                select_last(conn)
            })
    })
}
