use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};
use tracing::{debug, error, warn};

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
    retention_days: i64,
) -> Result<Artifacts> {
    debug!("inserting artifact to the database");

    let date_created = Utc::now();
    let date_expires = expiration_date(date_created, retention_days);
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

fn expiration_date(date_created: DateTime<Utc>, retention_days: i64) -> DateTime<Utc> {
    let expires = if retention_days > 0 {
        Duration::try_days(retention_days).and_then(|x| date_created.checked_add_signed(x))
    } else {
        None
    };

    expires.unwrap_or_else(|| {
        warn!(
            "invalid artifacts retention of {retention_days} days, falling back to {DEFAULT_RETENTION_DAYS} days"
        );
        date_created + Duration::days(DEFAULT_RETENTION_DAYS)
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

#[cfg(test)]
mod tests {
    use super::{DEFAULT_RETENTION_DAYS, expiration_date};
    use chrono::{Duration, Utc};

    #[test]
    fn expiration_date_uses_the_configured_retention() {
        let date_created = Utc::now();
        assert_eq!(
            expiration_date(date_created, 30),
            date_created + Duration::days(30)
        );
    }

    #[test]
    fn expiration_date_falls_back_for_non_positive_retention() {
        let date_created = Utc::now();
        let expected = date_created + Duration::days(DEFAULT_RETENTION_DAYS);
        assert_eq!(expiration_date(date_created, 0), expected);
        assert_eq!(expiration_date(date_created, -1), expected);
    }

    #[test]
    fn expiration_date_falls_back_for_out_of_range_retention() {
        let date_created = Utc::now();
        let expected = date_created + Duration::days(DEFAULT_RETENTION_DAYS);
        assert_eq!(expiration_date(date_created, i64::MAX), expected);
        assert_eq!(expiration_date(date_created, 100_000_000), expected);
    }
}
