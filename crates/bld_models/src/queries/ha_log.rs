use anyhow::{anyhow, Result};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, ConnectionTrait, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::high_availability_log::Model as HighAvailLog;
use crate::generated::high_availability_log::{self, Entity as HighAvailLogEntity};

pub const BLANK: &str = "blank";
pub const NORMAL: &str = "normal";
pub const CONFIG_CHANGE: &str = "config_change";
pub const SNAPSHOT_POINTER: &str = "snapshot_pointer";

#[derive(Debug)]
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

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(conn: &C) -> Result<HighAvailLog> {
    debug!("loading the last entry of high availability log");

    let model = HighAvailLogEntity::find()
        .order_by_desc(high_availability_log::Column::Id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability log due to: not found");
            anyhow!("high availability log not found")
        })
        .inspect(|_| {
            debug!("loaded high availability log successfully");
        })
}

pub async fn select_last_rows<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    rows: u64,
) -> Result<Vec<HighAvailLog>> {
    debug!("loading the last {} rows of high availability log", rows);

    HighAvailLogEntity::find()
        .order_by_desc(high_availability_log::Column::Id)
        .limit(rows)
        .all(conn)
        .await
        .inspect(|_| {
            debug!("loaded high availability logs successfully");
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    lg_id: i32,
) -> Result<HighAvailLog> {
    debug!("loading high availability log with id: {}", lg_id);

    let model = HighAvailLogEntity::find_by_id(lg_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability log due to: not found");
            anyhow!("high availability log not found")
        })
        .inspect(|_| {
            debug!("loaded high availability log successfully");
        })
}

pub async fn select_between_ids<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    lg_start_id: i32,
    lg_end_id: i32,
) -> Result<Vec<HighAvailLog>> {
    debug!(
        "loading high availability logs from id: {} to id: {}",
        lg_start_id, lg_end_id
    );
    HighAvailLogEntity::find()
        .filter(high_availability_log::Column::Id.gte(lg_start_id))
        .filter(high_availability_log::Column::Id.lte(lg_end_id))
        .all(conn)
        .await
        .inspect(|_| {
            debug!("loaded high availability logs successfully");
        })
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_by_payload_type<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailLog> {
    debug!("loading high availability log with either config_change or snapshot payload types");

    let model = HighAvailLogEntity::find()
        .filter(
            Condition::any()
                .add(high_availability_log::Column::PayloadType.eq(CONFIG_CHANGE))
                .add(high_availability_log::Column::PayloadType.eq(SNAPSHOT_POINTER)),
        )
        .order_by_desc(high_availability_log::Column::DateCreated)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability log due to: not found");
            anyhow!("high availability log not found")
        })
        .inspect(|_| {
            debug!("load high availability log model entries successfully");
        })
}

pub async fn select_first_by_date_created_desc<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailLog> {
    debug!("loading first high availability log ordered by descending creation date");

    let model = HighAvailLogEntity::find()
        .order_by_desc(high_availability_log::Column::DateCreated)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability log due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability log due to: not found");
            anyhow!("high availability log not found")
        })
        .inspect(|_| {
            debug!("loaded first high availability log successfully");
        })
}

pub async fn select_config_greater_than_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    lg_id: i32,
) -> Result<HighAvailLog> {
    debug!(
        "loading high availability logs with greater id than: {}",
        lg_id
    );

    let model = HighAvailLogEntity::find()
        .filter(high_availability_log::Column::Id.gte(lg_id))
        .filter(high_availability_log::Column::PayloadType.eq(CONFIG_CHANGE))
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability logs due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability log due to: not found");
            anyhow!("high availability log not found")
        })
        .inspect(|_| {
            debug!("loaded high availability logs successfully");
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertHighAvailLog,
) -> Result<()> {
    debug!("inserting new high availability log: {:?}", model);

    let model = high_availability_log::ActiveModel {
        id: Set(model.id),
        term: Set(model.term),
        payload: Set(model.payload),
        payload_type: Set(model.payload_type),
        date_created: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    model.insert(conn).await.map_err(|e| {
        error!("could not insert high availability log due to: {}", e);
        anyhow!(e)
    })?;

    debug!("high availability log inserted successfully");
    Ok(())
}

pub async fn insert_many<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    models: Vec<InsertHighAvailLog>,
) -> Result<()> {
    debug!("inserting multiple high availability log entries");

    let models: Vec<high_availability_log::ActiveModel> = models
        .into_iter()
        .map(|m| high_availability_log::ActiveModel {
            id: Set(m.id),
            term: Set(m.term),
            payload: Set(m.payload),
            payload_type: Set(m.payload_type),
            ..Default::default()
        })
        .collect();

    HighAvailLogEntity::insert_many(models)
        .exec(conn)
        .await
        .map_err(|e| {
            error!(
                "could not insert multiple high availability logs due to: {}",
                e
            );
            anyhow!(e)
        })?;

    debug!("inserted multiple high availability log entries successfully");
    Ok(())
}

pub async fn delete<C: ConnectionTrait + TransactionTrait>(conn: &C) -> Result<()> {
    debug!("deleting all high availability logs");
    HighAvailLogEntity::delete_many()
        .exec(conn)
        .await
        .map(|_| debug!("deleted all high availability logs successfully"))
        .map_err(|e| {
            error!("could not delete high availability logs due to: {}", e);
            anyhow!(e)
        })
}

pub async fn delete_by_ids<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    lg_ids: Vec<i32>,
) -> Result<()> {
    debug!("deleting high availability log entries");
    let txn = conn.begin().await?;

    for lg_id in lg_ids {
        HighAvailLogEntity::delete_many()
            .filter(high_availability_log::Column::Id.eq(lg_id))
            .exec(conn)
            .await
            .map(|_| debug!("deleted high availability log entries successfully"))
            .map_err(|e| {
                error!(
                    "could not delete high availability log entries due to: {}",
                    e
                );
                anyhow!(e)
            })?;
    }

    txn.commit().await?;
    Ok(())
}

pub async fn delete_from_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    lg_id: i32,
) -> Result<()> {
    debug!(
        "deleting high availability log entries starting from id: {}",
        lg_id
    );
    HighAvailLogEntity::delete_by_id(lg_id)
        .exec(conn)
        .await
        .map(|_| debug!("deleted high availability log entry successfully"))
        .map_err(|e| {
            error!("could not delete high availability log entry due to: {}", e);
            anyhow!(e)
        })
}

pub async fn delete_until_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    lg_id: i32,
) -> Result<()> {
    debug!(
        "deleting high availability logs less than equal to: {}",
        lg_id
    );
    HighAvailLogEntity::delete_many()
        .filter(high_availability_log::Column::Id.lt(lg_id))
        .exec(conn)
        .await
        .map(|_| debug!("deleted high availability logs successfully"))
        .map_err(|e| {
            error!("could not delete high availability logs due to: {}", e);
            anyhow!(e)
        })
}
