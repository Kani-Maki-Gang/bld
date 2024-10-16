use anyhow::{anyhow, Result};
use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::high_availability_members::Model as HighAvailMembers;
use crate::generated::{
    high_availability_members::{self, Entity as HighAvailMembersEntity},
    high_availability_snapshot,
};

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

pub async fn select<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sn_id: i32,
) -> Result<Vec<HighAvailMembers>> {
    debug!(
        "loading high availability members of snapshot with id: {}",
        sn_id
    );
    HighAvailMembersEntity::find()
        .join(
            JoinType::InnerJoin,
            high_availability_members::Relation::HighAvailabilitySnapshot.def(),
        )
        .filter(high_availability_snapshot::Column::Id.eq(sn_id))
        .all(conn)
        .await
        .inspect(|_| {
            debug!(
                "loaded high availability members of snapshot with id: {} successfully",
                sn_id
            );
        })
        .map_err(|e| {
            error!("could not load high availability members due to: {}", e);
            anyhow!(e)
        })
}

pub async fn select_last_rows<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    rows: u64,
) -> Result<Vec<HighAvailMembers>> {
    debug!("loading last {} rows of high availability members", rows);
    HighAvailMembersEntity::find()
        .order_by_desc(high_availability_members::Column::Id)
        .limit(rows)
        .all(conn)
        .await
        .inspect(|_| {
            debug!("loaded high availability members successfully");
        })
        .map_err(|e| {
            error!("could not load high availability members due to: {}", e);
            anyhow!(e)
        })
}

pub async fn insert_many<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    models: Vec<InsertHighAvailMembers>,
) -> Result<()> {
    debug!("inserting multiple high availability members");

    let models: Vec<high_availability_members::ActiveModel> = models
        .into_iter()
        .map(|m| high_availability_members::ActiveModel {
            id: Set(m.id),
            snapshot_id: Set(m.snapshot_id),
            date_created: Set(Utc::now().naive_utc()),
            ..Default::default()
        })
        .collect();

    HighAvailMembersEntity::insert_many(models)
        .exec(conn)
        .await
        .map_err(|e| {
            error!("could not insert high availability members due to: {}", e);
            anyhow!(e)
        })?;

    debug!("inserted multiple high availability members successfully");
    Ok(())
}
