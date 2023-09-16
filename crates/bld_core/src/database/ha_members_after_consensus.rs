use anyhow::{anyhow, Result};
use bld_entities::{
    high_availability_members_after_consensus::{
        self, Entity as HighAvailMembersAfterConsensusEntity,
    },
    high_availability_snapshot,
};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, TransactionTrait,
};
use tracing::{debug, error};

pub use bld_entities::high_availability_members_after_consensus::Model as HighAvailMembersAfterConsensus;

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

pub async fn select<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sn_id: i32,
) -> Result<Vec<HighAvailMembersAfterConsensus>> {
    debug!(
        "loading high availability members after consensus of snapshot with id: {}",
        sn_id
    );

    HighAvailMembersAfterConsensusEntity::find()
        .join(
            JoinType::InnerJoin,
            high_availability_members_after_consensus::Relation::HighAvailabilitySnapshot.def(),
        )
        .filter(high_availability_snapshot::Column::Id.eq(sn_id))
        .all(conn)
        .await
        .map(|mc| {
            debug!(
                "loaded high availability members after consensus of snapshot: {} successfully",
                sn_id
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

pub async fn select_last_rows<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    rows: u64,
) -> Result<Vec<HighAvailMembersAfterConsensus>> {
    debug!(
        "loading the last {} rows high availability members after consensus",
        rows
    );
    HighAvailMembersAfterConsensusEntity::find()
        .order_by_desc(high_availability_members_after_consensus::Column::Id)
        .limit(rows)
        .all(conn)
        .await
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

pub async fn insert_many<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    models: Vec<InsertHighAvailMembersAfterConsensus>,
) -> Result<()> {
    debug!("inserting multiple high availability members after consensus");

    let models: Vec<high_availability_members_after_consensus::ActiveModel> = models
        .into_iter()
        .map(|m| high_availability_members_after_consensus::ActiveModel {
            id: Set(m.id),
            snapshot_id: Set(m.snapshot_id),
            ..Default::default()
        })
        .collect();

    HighAvailMembersAfterConsensusEntity::insert_many(models)
        .exec(conn)
        .await
        .map_err(|e| {
            error!(
                "could not insert high availability members after consensus due to: {}",
                e
            );
            anyhow!(e)
        })?;

    debug!("inserted multiple high availability members after consensus successfully");
    Ok(())
}
