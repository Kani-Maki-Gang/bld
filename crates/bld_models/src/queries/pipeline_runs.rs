use anyhow::{anyhow, Result};
use bld_migrations::Expr;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    prelude::DateTime, ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use tracing::{debug, error};

pub use crate::generated::pipeline_runs::Model as PipelineRuns;
use crate::generated::pipeline_runs::{self, Entity as PipelineRunsEntity};

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

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_id: &str,
) -> Result<PipelineRuns> {
    debug!("loading pipeline with id: {pip_id} from the database");

    let model = PipelineRunsEntity::find_by_id(pip_id)
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

pub async fn select_by_name<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_name: &str,
) -> Result<PipelineRuns> {
    debug!("loading pipeline with name: {pip_name} from the database");

    let model = PipelineRunsEntity::find()
        .filter(pipeline_runs::Column::Name.eq(pip_name))
        .order_by_desc(pipeline_runs::Column::DateCreated)
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

pub async fn select_last<C: ConnectionTrait + TransactionTrait>(conn: &C) -> Result<PipelineRuns> {
    debug!("loading the last invoked pipeline from the database");

    let model = PipelineRunsEntity::find()
        .order_by_desc(pipeline_runs::Column::DateCreated)
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

pub async fn select_with_filters<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
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
        .order_by_desc(pipeline_runs::Column::DateCreated)
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

pub async fn count_queued_on_date(conn: &DatabaseConnection, date: NaiveDateTime) -> Result<u64> {
    debug!("getting the count of pipelines that have been queued on date: {date}");
    PipelineRunsEntity::find()
        .filter(pipeline_runs::Column::State.ne(PR_STATE_INITIAL))
        .filter(pipeline_runs::Column::DateCreated.lte(date))
        .count(conn)
        .await
        .inspect(|_| debug!("got the count of pipelines that have been queued successfully"))
        .map_err(|e| {
            error!("could not get the count of running pipelines due to: {e}");
            anyhow!(e)
        })
}

pub async fn count_running(conn: &DatabaseConnection) -> Result<u64> {
    debug!("getting the count of currently running pipelines");
    PipelineRunsEntity::find()
        .filter(pipeline_runs::Column::State.eq(PR_STATE_RUNNING))
        .count(conn)
        .await
        .inspect(|_| debug!("got the count of running pipelines successfully"))
        .map_err(|e| {
            error!("could not get the count of running pipelines due to: {e}");
            anyhow!(e)
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    model: InsertPipelineRun,
) -> Result<()> {
    debug!("inserting new pipeline to the database");

    let active_model = pipeline_runs::ActiveModel {
        id: Set(model.id.to_owned()),
        name: Set(model.name.to_owned()),
        app_user: Set(model.app_user.to_owned()),
        state: Set(PR_STATE_INITIAL.to_owned()),
        date_created: Set(Utc::now().naive_utc()),
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

pub async fn update_state<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    id: &str,
    state: &str,
) -> Result<PipelineRuns> {
    debug!("updating pipeline id: {id} with values state: {state}");
    let current_date = Utc::now().naive_utc();
    let mut update_statement = PipelineRunsEntity::update_many()
        .col_expr(pipeline_runs::Column::State, Expr::value(state))
        .col_expr(
            pipeline_runs::Column::DateUpdated,
            Expr::value(current_date),
        );

    if state == PR_STATE_FINISHED || state == PR_STATE_FAULTED {
        update_statement =
            update_statement.col_expr(pipeline_runs::Column::EndDate, Expr::value(current_date));
    }

    update_statement
        .filter(pipeline_runs::Column::Id.eq(id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated pipeline successfully");
        })
        .map_err(|e| {
            error!("could not update pipeline run due to: {e}");
            anyhow!(e)
        })?;

    select_by_id(conn, id).await
}

pub async fn update_start_date<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    id: &str,
    start_date: &DateTime,
) -> Result<()> {
    debug!("updating pipeline run {id} with state_date {start_date}");
    PipelineRunsEntity::update_many()
        .col_expr(pipeline_runs::Column::StartDate, Expr::value(*start_date))
        .filter(pipeline_runs::Column::Id.eq(id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("update pipeline run start date successfully");
        })
        .map_err(|e| {
            debug!("couldn't update pipeline run's start date due to {e}");
            anyhow!(e)
        })
}
