use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::artifacts::Model as Artifacts;
use crate::generated::artifacts::{self, Entity as ArtifactsEntity};

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
        id: Set(uuid::Uuid::new_v4().to_string()),
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

pub async fn select_by_run_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    run_id: &str,
) -> Result<Vec<Artifacts>> {
    debug!("loading artifacts for run: {run_id}");

    ArtifactsEntity::find()
        .filter(artifacts::Column::RunId.eq(run_id))
        .all(conn)
        .await
        .map_err(|e| {
            error!("could not load artifacts due to: {e}");
            anyhow!(e)
        })
}

pub async fn select_expired<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<Vec<Artifacts>> {
    debug!("loading expired artifacts");

    ArtifactsEntity::find()
        .filter(artifacts::Column::DateExpires.lt(Utc::now().naive_utc()))
        .all(conn)
        .await
        .map_err(|e| {
            error!("could not load expired artifacts due to: {e}");
            anyhow!(e)
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    id: &str,
) -> Result<Artifacts> {
    debug!("loading artifact with id: {id}");

    ArtifactsEntity::find_by_id(id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load artifact due to: {e}");
            anyhow!(e)
        })?
        .ok_or_else(|| {
            error!("couldn't load artifact. Not found");
            anyhow!("artifact not found")
        })
}

pub async fn delete_by_id<C: ConnectionTrait + TransactionTrait>(conn: &C, id: &str) -> Result<()> {
    debug!("deleting artifact with id: {id}");

    ArtifactsEntity::delete_by_id(id)
        .exec(conn)
        .await
        .map(|_| {
            debug!("deleted artifact successfully");
        })
        .map_err(|e| {
            error!("could not delete artifact due to: {e}");
            anyhow!(e)
        })
}
