use anyhow::{anyhow, Result};
use bld_migrations::Expr;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::high_availability_hard_state::Model as HighAvailHardState;
use crate::generated::high_availability_hard_state::{self, Entity as HighAvailHardStateEntity};

#[derive(Debug)]
pub struct InsertHighAvailHardState {
    pub current_term: i32,
    pub voted_for: Option<i32>,
}

impl InsertHighAvailHardState {
    pub fn new(hs_current_term: i32, hs_voted_for: Option<i32>) -> Self {
        Self {
            current_term: hs_current_term,
            voted_for: hs_voted_for,
        }
    }
}

pub async fn select_first<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailHardState> {
    debug!("loading the first high availability hard state");

    let model = HighAvailHardStateEntity::find()
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load the first high availability hard state due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load the first high availability hard state due to: not found");
            anyhow!("high availability hard state not found")
        })
        .map(|ha| {
            debug!("loaded the first high availability hard state successfully");
            ha
        })
}

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailHardState> {
    debug!("loading the last entry of high availability hard state");
    let model = HighAvailHardStateEntity::find()
        .order_by_desc(high_availability_hard_state::Column::Id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availabiilty hard state due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability hard state due to: not found");
            anyhow!("high availability hard state not found")
        })
        .map(|hs| {
            debug!("loaded high availability hard state successfully");
            hs
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    hs_id: i32,
) -> Result<HighAvailHardState> {
    debug!("loading high availability hard state with id: {}", hs_id);

    let model = HighAvailHardStateEntity::find_by_id(hs_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load high availability hard due to: {}", e);
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability hard state due to: not found");
            anyhow!("high availability hard state not found")
        })
        .map(|hs| {
            debug!("loaded high availability hard state successfully");
            hs
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertHighAvailHardState,
) -> Result<()> {
    debug!("inserting new high availability hard state: {:?}", model);

    let active_model = high_availability_hard_state::ActiveModel {
        current_term: Set(model.current_term),
        voted_for: Set(model.voted_for),
        date_created: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    active_model.insert(conn).await.map_err(|e| {
        error!(
            "insert failed for high availability hard state due to: {}",
            e
        );
        anyhow!(e)
    })?;

    debug!("high avail hard state inserted successfully");
    Ok(())
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    id: i32,
    current_term: i32,
    voted_for: Option<i32>,
) -> Result<()> {
    debug!("updateing the high availability hard state with id: {}", id);
    let date_updated = Utc::now().naive_utc();
    HighAvailHardStateEntity::update_many()
        .col_expr(
            high_availability_hard_state::Column::CurrentTerm,
            Expr::value(current_term),
        )
        .col_expr(
            high_availability_hard_state::Column::VotedFor,
            Expr::value(voted_for),
        )
        .col_expr(
            high_availability_hard_state::Column::DateUpdated,
            Expr::value(date_updated),
        )
        .filter(high_availability_hard_state::Column::Id.eq(id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated the high availability hard state successfully");
        })
        .map_err(|e| {
            error!(
                "update of high availability hard state failed due to: {}",
                e
            );
            anyhow!(e)
        })
}
