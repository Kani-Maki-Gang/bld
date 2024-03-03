use anyhow::{anyhow, Result};
use bld_migrations::Expr;
use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::high_availability_client_status::Model as HighAvailClientStatus;
use crate::generated::high_availability_client_status::{
    self, Entity as HighAvailClientStatusEntity,
};

pub struct InsertHighAvailClientStatus {
    pub id: i32,
    pub state_machine_id: i32,
    pub status: String,
}

impl InsertHighAvailClientStatus {
    pub fn new(cs_id: i32, cs_state_machine_id: i32, cs_status: String) -> Self {
        Self {
            id: cs_id,
            state_machine_id: cs_state_machine_id,
            status: cs_status,
        }
    }
}

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailClientStatus> {
    debug!("loading last entry of high availability client status");
    let model = HighAvailClientStatusEntity::find()
        .order_by_desc(high_availability_client_status::Column::Id)
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability client status due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability client status due to: not found");
            anyhow!("high availability client status not found")
        })
        .map(|cs| {
            debug!("loaded high availability client status successfully");
            cs
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cs_id: i32,
) -> Result<HighAvailClientStatus> {
    debug!(
        "loading high availability client status model with id: {}",
        cs_id
    );

    let model = HighAvailClientStatusEntity::find_by_id(cs_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability client status due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability client status due to: not found");
            anyhow!("high availability client status not found")
        })
        .map(|cs| {
            debug!("loaded high availability client status successfully");
            cs
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertHighAvailClientStatus,
) -> Result<()> {
    debug!(
        "inserting high availability client status with status: {} for state machine: {}",
        model.status, model.state_machine_id
    );

    let model = high_availability_client_status::ActiveModel {
        id: Set(model.id),
        state_machine_id: Set(model.state_machine_id),
        status: Set(model.status),
        date_created: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    HighAvailClientStatusEntity::insert(model)
        .exec(conn)
        .await
        .map_err(|e| {
            error!(
                "could not insert high availability client status due to: {}",
                e
            );
            anyhow!(e)
        })?;

    debug!("inserted high availability client status successfully");
    Ok(())
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    id: i32,
    status: &str,
) -> Result<()> {
    debug!(
        "updating high availability client status: {} with status: {}",
        id, status
    );
    let date_updated = Utc::now().naive_utc();
    HighAvailClientStatusEntity::update_many()
        .col_expr(
            high_availability_client_status::Column::Status,
            Expr::value(status),
        )
        .col_expr(
            high_availability_client_status::Column::DateUpdated,
            Expr::value(date_updated),
        )
        .filter(high_availability_client_status::Column::Id.eq(id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated high availability client status successfully");
        })
        .map_err(|e| {
            error!(
                "could not update high availability client status due to: {}",
                e
            );
            anyhow!(e)
        })
}
