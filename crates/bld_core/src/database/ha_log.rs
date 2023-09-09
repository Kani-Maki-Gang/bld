use anyhow::{anyhow, Result};
use bld_entities::high_availability_log;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, Condition};
use tracing::{debug, error};

pub use bld_entities::high_availability_log::Entity as HighAvailLog;

pub const BLANK: &str = "blank";
pub const NORMAL: &str = "normal";
pub const CONFIG_CHANGE: &str = "config_change";
pub const SNAPSHOT_POINTER: &str = "snapshot_pointer";

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
}

pub async fn select_last(conn: &DatabaseConnection) -> Result<HighAvailLog> {
    debug!("loading the last entry of high availability log");
    HighAvailLog::find()
        .order_by_desc(high_availability_log::Column::Id)
        .one(conn)
        .await
        .map(|l| {
            debug!("loaded high availability log successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_last_rows(conn: &DatabaseConnection, rows: i64) -> Result<Vec<HighAvailLog>> {
    debug!("loading the last {} rows of high availability log", rows);
    HighAvailLog::find()
        .order_by_desc(high_availability_log::Column::Id).
        .limit(rows)
        .load(conn)
        .await
        .map(|l| {
            debug!("loaded high availability logs successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_by_id(conn: &DatabaseConnection, lg_id: i32) -> Result<HighAvailLog> {
    debug!("loading high availability log with id: {}", lg_id);
    HighAvailLog::find_by_id(lg_id)
        .one(conn)
        .await
        .map(|l| {
            debug!("loaded high availability log successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_between_ids(
    conn: &DatabaseConnection,
    lg_start_id: i32,
    lg_end_id: i32,
) -> Result<Vec<HighAvailLog>> {
    debug!(
        "loading high availability logs from id: {} to id: {}",
        lg_start_id, lg_end_id
    );
    HighAvailLog::find()
        .filter(high_availability_log::Column::Id.ge(lg_start_id))
        .filter(high_availability_log::Column::Id.le(lg_end_id))
        .load(conn)
        .await
        .map(|l| {
            debug!("loaded high availability logs successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_by_payload_type(conn: &DatabaseConnection) -> Result<HighAvailLog> {
    debug!("loading high availability log with either config_change or snapshot payload types");
    HighAvailLog::find()
        .filter(
            Condition::any()
                .add(high_availability_log::Column::PayloadType.eq(CONFIG_CHANGE))
                .add(high_availability_log::Column::PayloadType.eq(SNAPSHOT_POINTER))
        )
        .order_by_desc(high_availability_log::Column::DateCreated)
        .one(conn)
        .await
        .map(|l| {
            debug!("load high availability log model entries successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_first_by_date_created_desc(conn: &DatabaseConnection) -> Result<HighAvailLog> {
    debug!("loading first high availability log ordered by descending creation date");
    HighAvailLog::find()
        .order_by_desc(high_availability_log::Column::DateCreated)
        .one(conn)
        .await
        .map(|l| {
            debug!("loaded first high availability log successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_config_greater_than_id(
    conn: &DatabaseConnection,
    lg_id: i32,
) -> Result<HighAvailLog> {
    debug!(
        "loading high availability logs with greater id than: {}",
        lg_id
    );
    HighAvailLog::find()
        .filter(high_availability_log::Column::Id.gt(lg_id))
        .filter(high_availability_log::Column::PayloadType.eq(CONFIG_CHANGE))
        .one(conn)
        .await
        .map(|l| {
            debug!("loaded high availability logs successfully");
            l
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn insert(conn: &DatabaseConnection, model: InsertHighAvailLog) -> Result<HighAvailLog> {
    debug!("inserting new high availability log: {:?}", model);

    let model = high_availability_log::ActiveModel {
        id: model.id,
        term: model.term,
        payload: model.payload,
        payload_type: model.payload_type,
        ..Default::default()
    };

    model
        .insert(conn)
        .await
        .map_err(|e| {
            error!("could not insert high availability log due to: {}", e);
            anyhow!(e)
        })?;

    debug!("high availability log inserted successfully");
    model
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
