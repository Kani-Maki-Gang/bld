#![allow(dead_code, unused_imports)]

use crate::persist::db::schema::ha_hard_state;
use crate::persist::db::schema::ha_hard_state::dsl::*;
use anyhow::anyhow;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::Queryable;
use tracing::debug;

#[derive(Debug, Queryable)]
pub struct HighAvailHardStateModel {
    pub id: String,
    pub current_term: i32,
    pub voted_for: Option<i32>,
    pub date_created: String,
    pub date_updated: String,
}

impl HighAvailHardStateModel {
    pub fn select(conn: &SqliteConnection, hs_id: &str) -> anyhow::Result<Self> {
        debug!("loading high availability hard state with id: {}", hs_id);
        ha_hard_state
            .filter(id.eq(hs_id))
            .first::<Self>(conn)
            .map_err(|e| {
                debug!("could not load high availability hard due to: {}", e);
                anyhow!(e)
            })
    }
}
