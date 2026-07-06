use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, TransactionTrait};
use tracing::{debug, error};

pub use crate::generated::artifacts::Model as Artifacts;
use crate::generated::artifacts;

const DEFAULT_RETENTION_DAYS: i64 = 7;

pub struct InsertArtifact {
    pub run_id: String,
    pub name: String,
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertArtifact,
) -> Result<Artifacts> {
    debug!("inserting artifact to the database");

    let date_created = Utc::now();
    let date_expires = date_created + Duration::days(DEFAULT_RETENTION_DAYS);
    let active_model = artifacts::ActiveModel {
        run_id: Set(model.run_id),
        name: Set(model.name),
        date_created: Set(date_created.naive_utc()),
        date_expires: Set(date_expires.naive_utc()),
        ..Default::default()
    };

    active_model.insert(conn).await.map_err(|e| {
        error!("could not insert artifact due to: {e}");
        anyhow!(e)
    })
}
