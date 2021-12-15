#![allow(dead_code, unused_imports)]

use crate::persist::db::schema::ha_log;
use crate::persist::db::schema::ha_log::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable};
use tracing::debug;

#[derive(Debug, Queryable)]
pub struct HighAvailLogModel {
    pub id: i32,
    pub term: i32,
    pub idx: i32,
    pub payload_type: String,
    pub payload: String,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailLogModel {
    pub fn select(conn: &SqliteConnection, lg_id: i32) -> anyhow::Result<Self> {
        debug!("loading high availability log model with id: {}", lg_id);
        ha_log
            .filter(id.eq(lg_id))
            .first::<Self>(conn)
            .map_err(|e| {
                debug!("could not load high availability log due to: {}", lg_id);
                anyhow!(e)
            })
    }
}
