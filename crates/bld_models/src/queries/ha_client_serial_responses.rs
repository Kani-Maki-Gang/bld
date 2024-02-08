use anyhow::{anyhow, Result};
use chrono::Utc;
use sea_orm::{ActiveValue::Set, ConnectionTrait, EntityTrait, QueryOrder, TransactionTrait};
use tracing::{debug, error};

pub use crate::generated::high_availability_client_serial_responses::Model as HighAvailClientSerialResponses;
use crate::generated::high_availability_client_serial_responses::{
    self, Entity as HighAvailClientSerialResponsesEntity,
};

#[derive(Debug)]
pub struct InsertHighAvailClientSerialResponses {
    pub id: i32,
    pub state_machine_id: i32,
    pub serial: i32,
    pub response: Option<String>,
}

impl InsertHighAvailClientSerialResponses {
    pub fn new(csr_id: i32, csr_sm_id: i32, csr_serial: i32, csr_response: Option<&str>) -> Self {
        Self {
            id: csr_id,
            state_machine_id: csr_sm_id,
            serial: csr_serial,
            response: csr_response.map(|x| x.to_string()),
        }
    }
}

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<HighAvailClientSerialResponses> {
    debug!("loading the last high availability client serial response");

    let model = HighAvailClientSerialResponsesEntity::find()
        .order_by_desc(high_availability_client_serial_responses::Column::Id)
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability client serial response due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn'y load high availability client serial respone due to: not found");
            anyhow!("high availability client serial response not found")
        })
        .map(|csr| {
            debug!("loaded high availability client serial response successfully");
            csr
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    csr_id: i32,
) -> Result<HighAvailClientSerialResponses> {
    debug!(
        "loading high availability client serial response with id: {}",
        csr_id
    );

    let model = HighAvailClientSerialResponsesEntity::find_by_id(csr_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!(
                "could not load high availability client serial response due to: {}",
                e
            );
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load high availability client serial response due to: not found");
            anyhow!("high availability client serial response not found")
        })
        .map(|csr| {
            debug!("loaded high availability client serial response successfully");
            csr
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertHighAvailClientSerialResponses,
) -> Result<()> {
    debug!(
        "inserting high availability client serial responses model: {:?}",
        model
    );

    let model = high_availability_client_serial_responses::ActiveModel {
        id: Set(model.id),
        state_machine_id: Set(model.state_machine_id),
        serial: Set(model.serial),
        response: Set(model.response),
        date_created: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    HighAvailClientSerialResponsesEntity::insert(model)
        .exec(conn)
        .await
        .map_err(|e| {
            error!(
                "could not insert high availability client serial response due to: {}",
                e
            );
            anyhow!(e)
        })?;

    debug!("inserted high availability client serial responses successfully");
    Ok(())
}
