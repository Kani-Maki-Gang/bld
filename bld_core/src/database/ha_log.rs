use crate::database::schema::ha_log;
use crate::database::schema::ha_log::dsl::*;
use anyhow::{anyhow, Result};
use async_raft::raft::{Entry, EntryPayload};
use async_raft::AppData;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::sqlite::SqliteConnection;
use diesel::{Insertable, Queryable};
use tracing::{debug, error};

pub const BLANK: &str = "blank";
pub const NORMAL: &str = "normal";
pub const CONFIG_CHANGE: &str = "config_change";
pub const SNAPSHOT_POINTER: &str = "snapshot_pointer";

#[derive(Debug, Queryable)]
pub struct HighAvailLog {
    pub id: i32,
    pub term: i32,
    pub payload_type: String,
    pub payload: String,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = ha_log)]
pub struct InsertHighAvailLog {
    pub id: i32,
    pub term: i32,
    pub payload_type: String,
    pub payload: String,
}

impl InsertHighAvailLog {
    pub fn new(lg_id: i32, lg_term: i32, lg_payload_type: &str, lg_payload: Option<&str>) -> Self {
        Self {
            id: lg_id,
            term: lg_term,
            payload_type: lg_payload_type.to_string(),
            payload: match lg_payload {
                Some(p) => p.to_string(),
                None => String::new(),
            },
        }
    }

    fn from_entry<T: AppData>(entry: &Entry<T>) -> Self {
        Self {
            id: entry.index as i32,
            term: entry.term as i32,
            payload_type: match entry.payload {
                EntryPayload::Blank => BLANK.to_string(),
                EntryPayload::Normal(_) => NORMAL.to_string(),
                EntryPayload::ConfigChange(_) => CONFIG_CHANGE.to_string(),
                EntryPayload::SnapshotPointer(_) => SNAPSHOT_POINTER.to_string(),
            },
            payload: serde_json::to_string(&entry).unwrap(),
        }
    }
}

impl<T: AppData> From<Entry<T>> for InsertHighAvailLog {
    fn from(entry: Entry<T>) -> Self {
        Self::from_entry(&entry)
    }
}

impl<T: AppData> From<&Entry<T>> for InsertHighAvailLog {
    fn from(entry: &Entry<T>) -> Self {
        Self::from_entry(entry)
    }
}

pub fn select_last(conn: &mut SqliteConnection) -> Result<HighAvailLog> {
    debug!("loading the last entry of high availability log");
    ha_log
        .order(id.desc())
        .first(conn)
        .map(|l| {
            debug!("loaded high availability log successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_last_rows(conn: &mut SqliteConnection, rows: i64) -> Result<Vec<HighAvailLog>> {
    debug!("loading the last {} rows of high availability log", rows);
    ha_log
        .order(id.desc())
        .limit(rows)
        .load(conn)
        .map(|l| {
            debug!("loaded high availability logs successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_id(conn: &mut SqliteConnection, lg_id: i32) -> Result<HighAvailLog> {
    debug!("loading high availability log with id: {}", lg_id);
    ha_log
        .filter(id.eq(lg_id))
        .first(conn)
        .map(|l| {
            debug!("loaded high availability log successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_between_ids(
    conn: &mut SqliteConnection,
    lg_start_id: i32,
    lg_end_id: i32,
) -> Result<Vec<HighAvailLog>> {
    debug!(
        "loading high availability logs from id: {} to id: {}",
        lg_start_id, lg_end_id
    );
    ha_log
        .filter(id.ge(lg_start_id).and(id.le(lg_end_id)))
        .load(conn)
        .map(|l| {
            debug!("loaded high availability logs successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_by_payload_type(conn: &mut SqliteConnection) -> Result<HighAvailLog> {
    debug!("loading high availability log with either config_change or snapshot payload types");
    ha_log
        .filter(payload_type.eq(CONFIG_CHANGE))
        .or_filter(payload_type.eq(SNAPSHOT_POINTER))
        .order(date_created.desc())
        .first(conn)
        .map(|l| {
            debug!("load high availability log model entries successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_first_by_date_created_desc(conn: &mut SqliteConnection) -> Result<HighAvailLog> {
    debug!("loading first high availability log ordered by descending creation date");
    ha_log
        .order(date_created.desc())
        .first(conn)
        .map(|l| {
            debug!("loaded first high availability log successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })
}

pub fn select_config_greater_than_id(
    conn: &mut SqliteConnection,
    lg_id: i32,
) -> Result<HighAvailLog> {
    debug!(
        "loading high availability logs with greater id than: {}",
        lg_id
    );
    ha_log
        .filter(id.gt(lg_id).and(payload_type.eq(CONFIG_CHANGE)))
        .first(conn)
        .map(|l| {
            debug!("loaded high availability logs successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub fn insert(conn: &mut SqliteConnection, model: InsertHighAvailLog) -> Result<HighAvailLog> {
    debug!("inserting new high availability log: {:?}", model);
    conn.transaction(|conn| {
        diesel::insert_into(ha_log)
            .values(model)
            .execute(conn)
            .map_err(|e| {
                error!("could not insert high availability log due to: {}", e);
                anyhow!(e)
            })
            .and_then(|_| {
                debug!("high availability log inserted successfully");
                select_last(conn)
            })
    })
}

pub fn insert_many(
    conn: &mut SqliteConnection,
    models: Vec<InsertHighAvailLog>,
) -> Result<Vec<HighAvailLog>> {
    debug!("inserting multiple high availability log entries");
    conn.transaction(|conn| {
        diesel::insert_into(ha_log)
            .values(models)
            .execute(conn)
            .map_err(|e| {
                error!(
                    "could not insert multiple high availability logs due to: {}",
                    e
                );
                anyhow!(e)
            })
            .and_then(|rows| {
                debug!("inserted multiple high availability log entries successfully");
                select_last_rows(conn, rows as i64)
            })
    })
}

pub fn delete(conn: &mut SqliteConnection) -> Result<()> {
    debug!("deleting all high availability logs");
    diesel::delete(ha_log)
        .execute(conn)
        .map(|_| debug!("deleted all high availability logs successfully"))
        .map_err(|e| {
            error!("could not delete high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub fn delete_by_ids(conn: &mut SqliteConnection, lg_ids: Vec<i32>) -> Result<()> {
    debug!("deleting high availability log entries");
    diesel::delete(ha_log.filter(id.eq_any(lg_ids)))
        .execute(conn)
        .map(|_| debug!("deleted high availability log entries successfully"))
        .map_err(|e| {
            error!(
                "could not delete high availability log entries due to: {}",
                e
            );
            anyhow!(e)
        })
}

pub fn delete_from_id(conn: &mut SqliteConnection, lg_id: i32) -> Result<()> {
    debug!(
        "deleting high availability log entries starting from id: {}",
        lg_id
    );
    diesel::delete(ha_log.filter(id.ge(lg_id)))
        .execute(conn)
        .map(|_| debug!("deleted high availability log entry successfully"))
        .map_err(|e| {
            error!("could not delete high availability log entry due to: {}", e);
            anyhow!(e)
        })
}

pub fn delete_until_id(conn: &mut SqliteConnection, lg_id: i32) -> Result<()> {
    debug!(
        "deleting high availability logs less than equal to: {}",
        lg_id
    );
    diesel::delete(ha_log.filter(id.le(lg_id)))
        .execute(conn)
        .map(|_| debug!("deleted high availability logs successfully"))
        .map_err(|e| {
            error!("could not delete high availability logs due to: {}", e);
            anyhow!(e)
        })
}
