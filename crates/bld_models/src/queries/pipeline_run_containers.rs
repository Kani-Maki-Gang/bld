use anyhow::{anyhow, Result};
use bld_migrations::Expr;
use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, Condition, ConnectionTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait, TransactionTrait,
};
use tracing::{debug, error};

use super::pipeline_runs::{PR_STATE_FAULTED, PR_STATE_FINISHED};
pub use crate::generated::pipeline_run_containers::Model as PipelineRunContainers;
use crate::generated::{
    pipeline_run_containers::{self, Entity as PipelineRunContainersEntity},
    pipeline_runs,
};

pub const PRC_STATE_ACTIVE: &str = "active";
pub const PRC_STATE_REMOVED: &str = "removed";
pub const PRC_STATE_FAULTED: &str = "faulted";
pub const PRC_STATE_KEEP_ALIVE: &str = "keep-alive"; // Set when the pipeline is configured to not dispose.

#[derive(Debug)]
pub struct InsertPipelineRunContainer {
    pub id: String,
    pub run_id: String,
    pub container_id: String,
    pub state: String,
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    prc_id: &str,
) -> Result<PipelineRunContainers> {
    debug!("loading pipeline run container with id: {prc_id}");

    let model = PipelineRunContainersEntity::find_by_id(prc_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load pipeline run container. {e}");
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load pipeline run container. Not found");
            anyhow!("pipeline run container not found")
        })
        .inspect(|_| {
            debug!("loaded pipeline run container successfully");
        })
}

pub async fn select_in_invalid_state<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<Vec<PipelineRunContainers>> {
    debug!("loading all pipeline run containers that are in an invalid state");

    let mut active_containers = PipelineRunContainersEntity::find()
        .join(
            JoinType::InnerJoin,
            pipeline_run_containers::Relation::PipelineRuns.def(),
        )
        .filter(
            Condition::any()
                .add(pipeline_runs::Column::State.eq(PR_STATE_FINISHED))
                .add(pipeline_runs::Column::State.eq(PR_STATE_FAULTED)),
        )
        .filter(
            Condition::any()
                .add(pipeline_run_containers::Column::State.eq(PRC_STATE_ACTIVE))
                .add(pipeline_run_containers::Column::State.eq(PRC_STATE_FAULTED)),
        )
        .all(conn)
        .await
        .map_err(|e| {
            error!("couldn't load pipeline run containers due to {e}");
            anyhow!(e)
        })?;

    let mut faulted_containers = PipelineRunContainersEntity::find()
        .filter(pipeline_run_containers::Column::Id.eq(PRC_STATE_FAULTED))
        .all(conn)
        .await
        .inspect(|_| {
            debug!("loaded faulted pipeline run containers successfully");
        })
        .map_err(|e| {
            error!("could not load pipeline run containers, {e}");
            anyhow!(e)
        })?;

    faulted_containers.append(&mut active_containers);

    Ok(faulted_containers)
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertPipelineRunContainer,
) -> Result<PipelineRunContainers> {
    debug!("inserting pipeline run container");

    let id = model.id;
    let model = pipeline_run_containers::ActiveModel {
        id: Set(id.to_owned()),
        run_id: Set(model.run_id),
        container_id: Set(model.container_id),
        state: Set(model.state),
        date_created: Set(Utc::now().naive_utc()),
        date_updated: Set(Utc::now().naive_utc()),
    };

    PipelineRunContainersEntity::insert(model)
        .exec(conn)
        .await
        .map_err(|e| {
            error!("could not insert pipeline run container. {e}");
            anyhow!(e)
        })?;

    debug!("inserted pipeline run container successfully");
    select_by_id(conn, &id).await
}

pub async fn update_state<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    id: &str,
    state: &str,
) -> Result<()> {
    debug!("updating pipeline run container with id: {id} with new state: {state}");
    let date_updated = Utc::now().naive_utc();
    PipelineRunContainersEntity::update_many()
        .col_expr(pipeline_run_containers::Column::State, Expr::value(state))
        .col_expr(
            pipeline_run_containers::Column::DateUpdated,
            Expr::value(date_updated),
        )
        .filter(pipeline_run_containers::Column::Id.eq(id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated pipeline run containers successfully");
        })
        .map_err(|e| {
            error!("could not update pipeline run container. {e}");
            anyhow!(e)
        })
}

pub async fn update_running_containers_to_faulted<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    prc_run_id: &str,
) -> Result<()> {
    debug!(
        "updating all pipeline run containers of run id: {} from state running to faulted",
        prc_run_id
    );
    let date_updated = Utc::now().naive_utc();
    PipelineRunContainersEntity::update_many()
        .col_expr(
            pipeline_run_containers::Column::State,
            Expr::value(PRC_STATE_FAULTED),
        )
        .col_expr(
            pipeline_run_containers::Column::DateUpdated,
            Expr::value(date_updated),
        )
        .filter(pipeline_run_containers::Column::RunId.eq(prc_run_id))
        .filter(pipeline_run_containers::Column::State.eq(PRC_STATE_ACTIVE))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated pipeline run containers successfully");
        })
        .map_err(|e| {
            error!("could not update pipeline run containers, {e}");
            anyhow!(e)
        })
}
