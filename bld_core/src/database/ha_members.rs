use crate::database::schema::ha_members;
use crate::database::schema::ha_members::dsl::*;
use crate::database::ha_snapshot::HighAvailSnapshot;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Associations, Identifiable, Insertable, Queryable};
use tracing::{debug, error};

#[derive(Debug, Associations, Identifiable, Queryable)]
#[belongs_to(HighAvailSnapshot, foreign_key = "snapshot_id")]
#[table_name = "ha_members"]
pub struct HighAvailMembers {
    pub id: i32,
    pub snapshot_id: i32,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Insertable)]
#[table_name = "ha_members"]
pub struct InsertHighAvailMembers {
    pub id: i32,
    pub snapshot_id: i32,
}

impl InsertHighAvailMembers {
    pub fn new(m_id: i32, m_snapshot_id: i32) -> Self {
        Self {
            id: m_id,
            snapshot_id: m_snapshot_id,
        }
    }
}

pub fn select(
    conn: &SqliteConnection,
    sn: &HighAvailSnapshot,
) -> anyhow::Result<Vec<HighAvailMembers>> {
    debug!(
        "loading high availability members of snapshot with id: {}",
        sn.id
    );
    HighAvailMembers::belonging_to(sn)
        .load(conn)
        .map(|m| {
            debug!(
                "loaded high availability members of snapshot with id: {} successfully",
                sn.id
            );
            m
        })
        .map_err(|e| {
            error!("could not load high availability members due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_last_rows(
    conn: &SqliteConnection,
    rows: i64,
) -> anyhow::Result<Vec<HighAvailMembers>> {
    debug!("loading last {} rows of high availability members", rows);
    ha_members
        .order(id.desc())
        .limit(rows)
        .load(conn)
        .map(|m| {
            debug!("loaded high availability members successfully");
            m
        })
        .map_err(|e| {
            error!("could not load high availability members due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert_many(
    conn: &SqliteConnection,
    models: Vec<InsertHighAvailMembers>,
) -> anyhow::Result<Vec<HighAvailMembers>> {
    debug!("inserting multiple high availability members");
    conn.transaction(|| {
        diesel::insert_into(ha_members)
            .values(&models)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert high availability members due to: {}", e);
                anyhow!(e)
            })
            .and_then(|rows| {
                debug!("inserted multiple high availability members successfully");
                select_last_rows(conn, rows as i64)
            })
    })
}
