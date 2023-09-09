use anyhow::{anyhow, Result};
use bld_entities::pipeline;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use tracing::{debug, error};

use crate::database::cron_jobs;

pub use bld_entities::pipeline::Entity as Pipeline;

pub struct InsertPipeline {
    pub id: String,
    pub name: String,
}

pub async fn select_all(conn: &DatabaseConnection) -> Result<Vec<Pipeline>> {
    debug!("loading all pipelines from the database");
    Pipeline::find()
        .order_by_asc(pipeline::Column::Name)
        .load(conn)
        .await
        .map(|p| {
            debug!("loaded all pipelines successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipelines due to {e}");
            anyhow!(e)
        })
}

pub async fn select_by_id(conn: &DatabaseConnection, pip_id: &str) -> Result<Pipeline> {
    debug!("loading pipeline with id: {pip_id} from the database");
    Pipeline::find_by_id(pip_id)
        .one(conn)
        .await
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to {e}");
            anyhow!(e)
        })
}

pub async fn select_by_name(conn: &DatabaseConnection, pip_name: &str) -> Result<Pipeline> {
    debug!("loading pipeline with name: {pip_name} from the database");
    Pipeline::find()
        .filter(pipeline::Column::Name.eq(pip_name))
        .one(conn)
        .await
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to {e}");
            anyhow!(e)
        })
}

pub async fn update_name(conn: &DatabaseConnection, pip_id: &str, pip_name: &str) -> Result<()> {
    debug!("updating pipeline with id: {pip_id} with new name: {pip_name}");
    Pipeline::update_many()
        .set(pipeline::Column::Name.eq(pip_name))
        .filter(pipeline::Column::Id.eq(pip_id))
        .exec(conn)
        .await
        .map(|_| debug!("pipeline updated successfully"))
        .map_err(|e| {
            error!("could not update pipeline due to {e}");
            anyhow!(e)
        })
}

pub async fn insert(conn: &DatabaseConnection, model: InsertPipeline) -> Result<Pipeline> {
    debug!("inserting new pipeline to the database");

    let model = pipeline::ActiveModel {
        id: model.id,
        name: model.name,
        ..Default::default()
    };

    model
        .insert(conn)
        .await
        .map_err(|e| {
            error!("could not insert pipeline due to: {e}");
            anyhow!(e)
        })?;

    debug!(
        "created new pipeline entry with id: {}, name: {}",
        model.id, model.name
    );
    model
}

pub async fn delete_by_name(conn: &DatabaseConnection, pip_name: &str) -> Result<()> {
    debug!("deleting pipeline with name: {pip_name} from the database");
    conn.transaction(|txn| {
        Box::pin(async move {
            let model = select_by_name(txn, pip_name).await?;
            cron_jobs::delete_by_pipeline(txn, &model.id).await?;
            model
                .delete(txn)
                .await
                .map_err(|e| {
                    error!("could not delete pipeline due to {e}");
                    anyhow!(e)
                })
                .map(|_| {
                    debug!("pipeline deleted successfully");
                })
        })
    })
    .await
}
