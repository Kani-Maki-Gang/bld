use anyhow::{anyhow, Result};
use bld_entities::pipeline::{self, Entity as PipelineEntity};
use bld_migrations::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, ModelTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use tracing::{debug, error};

use crate::database::cron_jobs;

pub use bld_entities::pipeline::Model as Pipeline;

pub struct InsertPipeline {
    pub id: String,
    pub name: String,
}

pub async fn select_all<C: ConnectionTrait + TransactionTrait>(conn: &C) -> Result<Vec<Pipeline>> {
    debug!("loading all pipelines from the database");
    let models = PipelineEntity::find()
        .order_by_asc(pipeline::Column::Name)
        .all(conn)
        .await
        .map(|p| {
            debug!("loaded all pipelines successfully");
            p
        })
        .map_err(|e| {
            error!("couldn't load pipelines due to {e}");
            anyhow!(e.to_string())
        })?;
    Ok(models)
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_id: &str,
) -> Result<Pipeline> {
    debug!("loading pipeline with id: {pip_id} from the database");

    let model = PipelineEntity::find_by_id(pip_id)
        .one(conn)
        .await
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
        .map_err(|e| {
            error!("could not load pipeline due to {e}");
            anyhow!(e)
        })?;

    model.ok_or_else(|| {
        error!("couldn't load pipeline due to not found");
        anyhow!("pipeline not found")
    })
}

pub async fn select_by_name<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_name: &str,
) -> Result<Pipeline> {
    debug!("loading pipeline with name: {pip_name} from the database");

    let model = PipelineEntity::find()
        .filter(pipeline::Column::Name.eq(pip_name))
        .one(conn)
        .await
        .map_err(|e| {
            error!("couldn't load pipeline due to {e}");
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load pipeline due to not found");
            anyhow!("pipeline not found")
        })
        .map(|p| {
            debug!("loaded pipeline successfully");
            p
        })
}

pub async fn update_name<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_id: &str,
    pip_name: &str,
) -> Result<()> {
    debug!("updating pipeline with id: {pip_id} with new name: {pip_name}");
    PipelineEntity::update_many()
        .col_expr(pipeline::Column::Name, Expr::value(pip_name))
        .filter(pipeline::Column::Id.eq(pip_id))
        .exec(conn)
        .await
        .map(|_| debug!("pipeline updated successfully"))
        .map_err(|e| {
            error!("could not update pipeline due to {e}");
            anyhow!(e)
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertPipeline,
) -> Result<()> {
    debug!("inserting new pipeline to the database");

    let active_model = pipeline::ActiveModel {
        id: Set(model.id),
        name: Set(model.name),
        ..Default::default()
    };

    active_model
        .insert(conn)
        .await
        .map(|_| {
            debug!("created new pipeline entry successfully");
        })
        .map_err(|e| {
            error!("could not insert pipeline due to: {e}");
            anyhow!(e)
        })
}

pub async fn delete_by_name<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_name: &str,
) -> Result<()> {
    debug!("deleting pipeline with name: {pip_name} from the database");
    let txn = conn.begin().await?;
    let model = select_by_name(&txn, pip_name).await?;
    cron_jobs::delete_by_pipeline(&txn, &model.id).await?;
    model
        .delete(&txn)
        .await
        .map(|_| {
            debug!("pipeline deleted successfully");
        })
        .map_err(|e| {
            error!("could not delete pipeline due to {e}");
            anyhow!(e)
        })
}
