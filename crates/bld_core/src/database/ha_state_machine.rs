use anyhow::{anyhow, Result};
use bld_entities::high_availability_state_machine::{self, Entity as HighAvailStateMachineEntity};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryOrder, TransactionTrait,
};
use tracing::{debug, error};

use bld_entities::high_availability_state_machine::Model as HighAvailStateMachine;

pub async fn select_first<C: ConnectionTrait + TransactionTrait>(
    conn: &DatabaseConnection,
) -> Result<HighAvailStateMachine> {
    debug!("loading the first entry of high availability state machine");

    let model = HighAvailStateMachineEntity::find()
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability state machine due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability state machine due to: not found");
            anyhow!("high availability state machine not found")
        })
        .map(|sm| {
            debug!("loaded high availability state machine successfully");
            sm
        })
}

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(
    conn: &DatabaseConnection,
) -> Result<HighAvailStateMachine> {
    debug!("loading the last entry of high availability state machine");

    let model = HighAvailStateMachineEntity::find()
        .order_by_desc(high_availability_state_machine::Column::Id)
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability state machine due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability state machine due to: not found");
            anyhow!("high availability state machine not found")
        })
        .map(|sm| {
            debug!("loaded high availability state machine successfully");
            sm
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sm_id: i32,
) -> Result<HighAvailStateMachine> {
    debug!("loading high availability state machine with id: {}", sm_id);

    let model = HighAvailStateMachineEntity::find_by_id(sm_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability state machine due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability state machine due to: not found");
            anyhow!("high availability state machine not found")
        })
        .map(|sm| {
            debug!("loaded high availability state machine successfully");
            sm
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sm_last_applied_log: i32,
) -> Result<()> {
    debug!(
        "inserting high availability state machine with last applied log: {:?}",
        sm_last_applied_log
    );

    let model = high_availability_state_machine::ActiveModel {
        last_applied_log: Set(sm_last_applied_log),
        ..Default::default()
    };

    model.insert(conn).await.map_err(|e| {
        error!(
            "could not insert high availability state machine due to: {}",
            e
        );
        anyhow!(e)
    })?;

    debug!("inserted high availability state machine successfully");
    Ok(())
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sm_id: i32,
    sm_last_applied_log: i32,
) -> Result<()> {
    debug!(
        "updating high availability state machine with id: {}",
        sm_id
    );

    let mut model = select_by_id(conn, sm_id).await?.into_active_model();
    model.last_applied_log = Set(sm_last_applied_log);

    model.update(conn).await.map_err(|e| {
        error!(
            "could not update high availability state machine due to: {}",
            e
        );
        anyhow!(e)
    })?;

    debug!("updated high availability state machine successfully");
    Ok(())
}
