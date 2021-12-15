#![allow(dead_code, unused_imports)]

use crate::persist::HighAvailSnapshotModel;
use crate::persist::db::schema::ha_members_after_consensus;
use crate::persist::db::schema::ha_members_after_consensus::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Associations, Identifiable, Queryable)]
#[belongs_to(HighAvailSnapshotModel, foreign_key = "snapshot_id")]
#[table_name = "ha_members_after_consensus"]
pub struct HighAvailMembersAfterConsensusModel {
    pub id: i32,
    pub snapshot_id: i32,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailMembersAfterConsensusModel {
    pub fn select(conn: &SqliteConnection, sn: &HighAvailSnapshotModel) -> anyhow::Result<Vec<Self>> {
        debug!("loading high availability members after consensus of snapshot with id: {}", sn.id);
        HighAvailMembersAfterConsensusModel::belonging_to(sn)
            .load::<Self>(conn)
            .map(|mc| {
                debug!("loaded high availability members after consensus of snapshot: {} successfully", sn.id);
                mc
            })
            .map_err(|e| {
                debug!("could not load high availability members after consensus due to: {}", e);
                anyhow!(e)
            })
    }
}
