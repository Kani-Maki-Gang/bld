#![allow(dead_code, unused_imports)]

use crate::persist::HighAvailSnapshotModel;
use crate::persist::db::schema::ha_members;
use crate::persist::db::schema::ha_members::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Associations, Identifiable, Queryable)]
#[belongs_to(HighAvailSnapshotModel, foreign_key = "snapshot_id")]
#[table_name = "ha_members"]
pub struct HighAvailMembersModel {
    pub id: i32,
    pub snapshot_id: i32,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailMembersModel {
    pub fn select(conn: &SqliteConnection, sn: &HighAvailSnapshotModel) -> anyhow::Result<Vec<Self>> {
        debug!("loading high availability members of snapshot with id: {}", sn.id);
        HighAvailMembersModel::belonging_to(sn) 
            .load::<Self>(conn)
            .map(|m| {
                debug!("loaded high availability members of snapshot with id: {} successfully", sn.id);
                m
            })
            .map_err(|e| {
                debug!("could not load high availability members due to: {}", e);
                anyhow!(e)
            })
    }
}
