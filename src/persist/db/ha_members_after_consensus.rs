use crate::persist::db::schema::ha_members_after_consensus;
use crate::persist::db::schema::ha_members_after_consensus::dsl::*;
use crate::persist::ha_snapshot::HighAvailSnapshot;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use tracing::{debug, error};

#[derive(Debug, Associations, Identifiable, Queryable)]
#[belongs_to(HighAvailSnapshot, foreign_key = "snapshot_id")]
#[table_name = "ha_members_after_consensus"]
pub struct HighAvailMembersAfterConsensus {
    pub id: i32,
    pub snapshot_id: i32,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Insertable)]
#[table_name = "ha_members_after_consensus"]
pub struct InsertHighAvailMembersAfterConsensus {
    pub id: i32,
    pub snapshot_id: i32,
}

impl InsertHighAvailMembersAfterConsensus {
    pub fn new(mc_id: i32, mc_snapshot_id: i32) -> Self {
        Self {
            id: mc_id,
            snapshot_id: mc_snapshot_id,
        }
    }
}

pub fn select(
    conn: &SqliteConnection,
    sn: &HighAvailSnapshot,
) -> anyhow::Result<Vec<HighAvailMembersAfterConsensus>> {
    debug!(
        "loading high availability members after consensus of snapshot with id: {}",
        sn.id
    );
    HighAvailMembersAfterConsensus::belonging_to(sn)
        .load(conn)
        .map(|mc| {
            debug!(
                "loaded high availability members after consensus of snapshot: {} successfully",
                sn.id
            );
            mc
        })
        .map_err(|e| {
            error!(
                "could not load high availability members after consensus due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn select_last_rows(
    conn: &SqliteConnection,
    rows: i64,
) -> anyhow::Result<Vec<HighAvailMembersAfterConsensus>> {
    debug!(
        "loading the last {} rows high availability members after consensus",
        rows
    );
    ha_members_after_consensus
        .order(id.desc())
        .limit(rows)
        .load(conn)
        .map(|mc| {
            debug!("loaded high availability members after consensus successfully");
            mc
        })
        .map_err(|e| {
            error!(
                "could not load high availability members after consensus due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn insert_many(
    conn: &SqliteConnection,
    models: Vec<InsertHighAvailMembersAfterConsensus>,
) -> anyhow::Result<Vec<HighAvailMembersAfterConsensus>> {
    debug!("inserting multiple high availability members after consensus");
    conn.transaction(|| {
        diesel::insert_into(ha_members_after_consensus)
            .values(&models)
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not insert high availability members after consensus due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|rows| {
                debug!("inserted multiple high availability members after consensus successfully");
                select_last_rows(conn, rows as i64)
            })
    })
}
