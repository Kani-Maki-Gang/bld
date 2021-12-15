#![allow(dead_code, unused_imports)]

use crate::persist::db::schema::ha_snapshot;
use crate::persist::db::schema::ha_snapshot::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Identifiable, Queryable)]
#[table_name = "ha_snapshot"]
pub struct HighAvailSnapshotModel {
    pub id: i32,
    pub term: i32,
    pub data: Vec<u8>,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailSnapshotModel {
    pub fn select(conn: &SqliteConnection, sn_id: i32) -> anyhow::Result<Self> {
        debug!("loading high availability snapshot with id: {}", sn_id);
        ha_snapshot
            .filter(id.eq(sn_id))
            .first::<Self>(conn)
            .map(|sn| {
                debug!("loaded snapshot with id: {} successfully", sn_id);
                sn
            })
            .map_err(|e| {
                debug!("could not load high availability snapshot due to: {}", e);
                anyhow!(e)
            })
    }
}
