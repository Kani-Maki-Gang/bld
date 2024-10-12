use anyhow::{anyhow, Result};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, QueryOrder, TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::high_availability_snapshot::Model as HighAvailSnapshot;
use crate::generated::high_availability_snapshot::{self, Entity as HighAvailSnapshotEntity};

#[derive(Debug)]
pub struct InsertHighAvailSnapshot {
    pub id: i32,
    pub term: i32,
    pub data: Vec<u8>,
}

impl InsertHighAvailSnapshot {
    pub fn new(sn_id: i32, sn_term: i32, sn_data: Vec<u8>) -> Self {
        Self {
            id: sn_id,
            term: sn_term,
            data: sn_data,
        }
    }
}

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailSnapshot> {
    debug!("loading the last entry high availability snapshot");

    let model = HighAvailSnapshotEntity::find()
        .order_by_desc(high_availability_snapshot::Column::Id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability snapshot due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability snapshot due to: not found");
            anyhow!("high availability snapshot not found")
        })
        .inspect(|_| {
            debug!("loaded high availability snapshot successfully");
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertHighAvailSnapshot,
) -> Result<()> {
    debug!("inserting high availability snapshot: {:?}", model);

    let model = high_availability_snapshot::ActiveModel {
        id: Set(model.id),
        term: Set(model.term),
        data: Set(model.data),
        date_created: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    model.insert(conn).await.map_err(|e| {
        error!("could not insert high availability snapshot due to: {}", e);
        anyhow!(e)
    })?;

    debug!("inserted high availability snapshot successfully");
    Ok(())
}
