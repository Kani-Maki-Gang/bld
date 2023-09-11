use anyhow::{anyhow, Result};
use bld_entities::pipeline_runs::{self, Entity as PipelineRunsEntity};
use bld_migrations::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use tracing::{debug, error};

pub use bld_entities::pipeline_runs::Model as PipelineRuns;

pub const PR_STATE_INITIAL: &str = "initial";
pub const PR_STATE_QUEUED: &str = "queued";
pub const PR_STATE_RUNNING: &str = "running";
pub const PR_STATE_FINISHED: &str = "finished";
pub const PR_STATE_FAULTED: &str = "faulted";

pub struct InsertPipelineRun {
    pub id: String,
    pub name: String,
    pub app_user: String,
}

pub async fn select_all<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
) -> Result<Vec<PipelineRuns>> {
    debug!("loading all pipeline runs from the database");
    PipelineRunsEntity::find()
        .order_by_asc(pipeline_runs::Column::StartDate)
        .all(conn)
        .await
        .map(|p| {
            debug!("loaded all pipeline runs successfully");
            p
        })
        .map_err(|e| {
            error!("couldn't load pipeline runs due to: {e}");
            anyhow!(e)
        })
}

pub async fn select_running_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    run_id: &str,
) -> Result<PipelineRuns> {
    debug!("loading pipeline run with id: {run_id} that is in a running state");

    let model = PipelineRunsEntity::find()
        .filter(pipeline_runs::Column::Id.eq(run_id))
        .filter(pipeline_runs::Column::State.eq(PR_STATE_RUNNING))
        .one(conn)
        .await
        .map_err(|e| {
            error!("couldn't load pipeline run due to: {e}");
            anyhow!(e)
        })?
        .ok_or_else(|| {
            error!("couldn't load pipeline run due to: not found");
            anyhow!("pipeline run not found")
        })?;

    debug!("loaded pipeline runs successfully");
    Ok(model)
}

pub async fn select_by_id(conn: &DatabaseConnection, pip_id: &str) -> Result<PipelineRuns> {
    debug!("loading pipeline with id: {pip_id} from the database");

    let model = PipelineRunsEntity::find_by_id(pip_id)
        .one(conn)
        .await
        .map(|p| p)
        .map_err(|e| {
            error!("could not load pipeline run due to: {e}");
            anyhow!(e)
        })?
        .ok_or_else(|| {
            error!("couldn't load pipeline run due to: not found");
            anyhow!("pipeline run not found")
        })?;

    debug!("loaded pipeline successfully");
    Ok(model)
}

pub async fn select_by_name(conn: &DatabaseConnection, pip_name: &str) -> Result<PipelineRuns> {
    debug!("loading pipeline with name: {pip_name} from the database");

    let model = PipelineRunsEntity::find()
        .filter(pipeline_runs::Column::Name.eq(pip_name))
        .one(conn)
        .await
        .map_err(|e| {
            error!("could not load pipeline run due to: {e}");
            anyhow!(e)
        })?
        .ok_or_else(|| {
            error!("couldn't load pipeline run due to: not found");
            anyhow!("pipeline run not found")
        })?;

    debug!("loaded pipeline successfully");
    Ok(model)
}

pub async fn select_last(conn: &DatabaseConnection) -> Result<PipelineRuns> {
    debug!("loading the last invoked pipeline from the database");

    let model = PipelineRunsEntity::find()
        .order_by_desc(pipeline_runs::Column::StartDate)
        .one(conn)
        .await
        .map_err(|e| {
            error!("couldn't load pipeline run due to: {e}");
            anyhow!(e)
        })?
        .ok_or_else(|| {
            error!("couldn't load pipeline run due to: not found");
            anyhow!("pipeline run not found")
        })?;

    debug!("loaded pipeline successfully");
    Ok(model)
}

pub async fn select_with_filters(
    conn: &DatabaseConnection,
    flt_state: &Option<String>,
    flt_name: &Option<String>,
    limit_by: u64,
) -> anyhow::Result<Vec<PipelineRuns>> {
    debug!("loading pipeline runs from the database with filters:");

    let mut find = PipelineRunsEntity::find();

    if let Some(flt_state) = flt_state {
        find = find.filter(pipeline_runs::Column::State.eq(flt_state));
    }

    if let Some(flt_name) = flt_name {
        find = find.filter(pipeline_runs::Column::Name.eq(flt_name));
    }

    find.limit(limit_by)
        .order_by_desc(pipeline_runs::Column::StartDate)
        .all(conn)
        .await
        .map(|mut p| {
            debug!("loaded all pipeline runs successfully");
            p.reverse();
            p
        })
        .map_err(|e| {
            error!("could not load pipeline runs due to: {e}");
            anyhow!(e)
        })
}

pub async fn insert(conn: &DatabaseConnection, model: InsertPipelineRun) -> Result<()> {
    debug!("inserting new pipeline to the database");

    let active_model = pipeline_runs::ActiveModel {
        id: Set(model.id.to_owned()),
        name: Set(model.name.to_owned()),
        app_user: Set(model.app_user.to_owned()),
        state: Set(PR_STATE_INITIAL.to_owned()),
        ..Default::default()
    };

    active_model.insert(conn).await.map_err(|e| {
        error!("could not insert pipeline due to: {e}");
        anyhow!(e)
    })?;

    debug!(
        "created new pipeline run entry for id: {}, name: {}, user: {}",
        model.id, model.name, model.app_user
    );
    Ok(())
}

pub async fn update_state(
    conn: &DatabaseConnection,
    pip_id: &str,
    pip_state: &str,
) -> Result<PipelineRuns> {
    debug!("updating pipeline id: {pip_id} with values state: {pip_state}");

    PipelineRunsEntity::update_many()
        .col_expr(pipeline_runs::Column::State, Expr::value(pip_state))
        .filter(pipeline_runs::Column::Id.eq(pip_id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated pipeline successfully");
        })
        .map_err(|e| {
            error!("could not update pipeline run due to: {e}");
            anyhow!(e)
        })?;

    select_by_id(conn, pip_id).await
}
