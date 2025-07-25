use anyhow::{Result, anyhow};
use bld_migrations::Expr;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Statement, TransactionTrait, prelude::DateTime,
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

#[derive(Debug, FromQueryResult)]
pub struct PipelineCountPerState {
    pub initial: i64,
    pub queued: i64,
    pub running: i64,
    pub finished: i64,
    pub faulted: i64,
}

#[derive(Debug, FromQueryResult)]
pub struct PipelineRunsPerUser {
    pub count: i64,
    pub app_user: String,
}

#[derive(Debug, FromQueryResult)]
pub struct PipelinePerCompletedState {
    pub pipeline: String,
    pub finished_count: i64,
    pub faulted_count: i64,
}

#[derive(Debug, FromQueryResult)]
pub struct PipelineRunsPerMonth {
    pub month: i64,
    pub count: i64,
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

pub async fn count_queued(conn: &DatabaseConnection) -> Result<u64> {
    debug!("getting the count of pipelines that are queued");
    PipelineRunsEntity::find()
        .filter(pipeline_runs::Column::State.eq(PR_STATE_QUEUED))
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

pub async fn count_per_state_last_ten_days(
    conn: &DatabaseConnection,
) -> Result<PipelineCountPerState> {
    debug!("getting the count of pipelines per state in the last ten days");
    let previous_date = Utc::now().naive_utc() - Duration::days(10);
    let current_date = Utc::now().naive_utc();
    let query = match conn.get_database_backend() {
        DatabaseBackend::Postgres => {
            r#"
            select
                coalesce(sum(case when "state" = 'initial' then 1 else 0 end), 0) as "initial",
                coalesce(sum(case when "state" = 'queued' then 1 else 0 end), 0) as "queued",
                coalesce(sum(case when "state" = 'running' then 1 else 0 end), 0) as "running",
                coalesce(sum(case when "state" = 'finished' then 1 else 0 end), 0) as "finished",
                coalesce(sum(case when "state" = 'faulted' then 1 else 0 end), 0) as "faulted"
            from
                "pipeline_runs"
            where
                "date_created" between $1 and $2
        "#
        }

        DatabaseBackend::MySql => {
            r#"
            select
                coalesce(cast(sum(case when state = 'initial' then 1 else 0 end) as signed integer), 0) as initial,
                coalesce(cast(sum(case when state = 'queued' then 1 else 0 end) as signed integer), 0) as queued,
                coalesce(cast(sum(case when state = 'running' then 1 else 0 end) as signed integer), 0) as running,
                coalesce(cast(sum(case when state = 'finished' then 1 else 0 end) as signed integer), 0) as finished,
                coalesce(cast(sum(case when state = 'faulted' then 1 else 0 end) as signed integer), 0) as faulted
            from
                pipeline_runs
            where
                date_created >= ? and date_created <= ?
        "#
        }

        DatabaseBackend::Sqlite => {
            r#"
            select
                coalesce(cast(sum(case when state = 'initial' then 1 else 0 end) as bigint), 0) as initial,
                coalesce(cast(sum(case when state = 'queued' then 1 else 0 end) as bigint), 0) as queued,
                coalesce(cast(sum(case when state = 'running' then 1 else 0 end) as bigint), 0) as running,
                coalesce(cast(sum(case when state = 'finished' then 1 else 0 end) as bigint), 0) as finished,
                coalesce(cast(sum(case when state = 'faulted' then 1 else 0 end) as bigint), 0) as faulted
            from
                pipeline_runs
            where
                date_created >= ? and date_created <= ?
        "#
        }
    };

    PipelineCountPerState::find_by_statement(Statement::from_sql_and_values(
        conn.get_database_backend(),
        query,
        [previous_date.into(), current_date.into()],
    ))
    .one(conn)
    .await
    .inspect(|_| debug!("got the count of pipelines per state in the last ten days successfully"))
    .map_err(|e| {
        error!("could not get the count of pipelines per state in the last ten days due to: {e}");
        anyhow!(e)
    })
    .and_then(|v| v.ok_or_else(|| anyhow!("no row returned from query")))
}

pub async fn most_runs_per_user(conn: &DatabaseConnection) -> Result<Vec<PipelineRunsPerUser>> {
    debug!("getting the most runs per user in the last 30 days");
    let previous_date = (Utc::now() - Duration::days(30)).naive_utc();
    let current_date = Utc::now().naive_utc();
    let query = r#"
        select count(id) as count, app_user
        from pipeline_runs
        group by app_user
        order by count desc
    "#;
    PipelineRunsPerUser::find_by_statement(Statement::from_sql_and_values(
        conn.get_database_backend(),
        query,
        [previous_date.into(), current_date.into()],
    ))
    .all(conn)
    .await
    .inspect(|_| debug!("got the most runs per user in the last 30 days successfully"))
    .map_err(|e| {
        error!("could not get the most runs per user in the last 30 days due to: {e}");
        anyhow!(e)
    })
}

pub async fn select_per_completed_state(
    conn: &DatabaseConnection,
) -> Result<Vec<PipelinePerCompletedState>> {
    debug!("getting the success and failure rate of pipelines for the last 30 days");
    let previous_date = (Utc::now() - Duration::days(30)).naive_utc();
    let current_date = Utc::now().naive_utc();
    let query = match conn.get_database_backend() {
        DatabaseBackend::Postgres => {
            r#"
                select
                    p.name as pipeline,
                    sum(case when state = 'finished' then 1 else 0 end)  as finished_count,
                    sum(case when state = 'faulted' then 1 else 0 end) as faulted_count
                from
                    pipeline_runs as p
                where
                    date_created between $1 and $2
                group by
                    p.name
            "#
        }
        DatabaseBackend::MySql => {
            r#"
                select
                    p.name as pipeline,
                    cast(sum(case when state = 'finished' then 1 else 0 end) as signed integer) as finished_count,
                    cast(sum(case when state = 'faulted' then 1 else 0 end) as signed integer) as faulted_count
                from
                    pipeline_runs as p
                where
                    date_created >= ? and date_created <= ?
                group by
                    p.name
            "#
        }
        DatabaseBackend::Sqlite => {
            r#"
                select
                    p.name as pipeline,
                    cast(sum(case when state = 'finished' then 1 else 0 end) as bigint) as finished_count,
                    cast(sum(case when state = 'faulted' then 1 else 0 end) as bigint) as faulted_count
                from
                    pipeline_runs as p
                where
                    date_created >= ? and date_created <= ?
                group by
                    p.name
            "#
        }
    };
    PipelinePerCompletedState::find_by_statement(Statement::from_sql_and_values(
        conn.get_database_backend(),
        query,
        [previous_date.into(), current_date.into()],
    ))
    .all(conn)
    .await
    .inspect(|_| debug!("got the success and failure rate of pipelines for the last 30 days successfully"))
    .map_err(|e| {
        error!("could not get the success and failure rate of pipelines for the last 30 days due to: {e}");
        anyhow!(e)
    })
}

pub async fn runs_per_month(conn: &DatabaseConnection) -> Result<Vec<PipelineRunsPerMonth>> {
    debug!("getting the count of pipeline runs per month");
    let query = match conn.get_database_backend() {
        DatabaseBackend::Postgres => {
            r#"
            with Data as (
                select
                    extract(month from p.date_created) as month,
                    count(p.id) as count
                from
                    pipeline_runs as p
                where
                    p.state in ('running', 'finished', 'faulted')
                group by
                    month
                order by
                    month
            ),
            Months as (
                select extract(month from generate_series('2024-01-01'::Date, '2024-12-31'::Date, '1 month')) as month
            )
            select
                cast(m.month as bigint) as month,
                case when d.count is null then 0 else d.count end as count
            from
                Months as m
                left join Data as d
                    on m.month = d.month;
            "#
        }
        DatabaseBackend::MySql => {
            r#"
            with Data as (
                select
                    extract(month from p.date_created) as month,
                    count(p.id) as count
                from
                    pipeline_runs as p
                where
                    p.state in ('running', 'finished', 'faulted')
                group by
                    month
                order by
                    month
            ),
            Months as (
                select 1 as month
                union all
                select 2 as month
                union all
                select 3 as month
                union all
                select 4 as month
                union all
                select 5 as month
                union all
                select 6 as month
                union all
                select 7 as month
                union all
                select 8 as month
                union all
                select 9 as month
                union all
                select 10 as month
                union all
                select 11 as month
                union all
                select 12 as month
            )
            select
                m.month,
                case when d.count is null then 0 else d.count end as count
            from
                Months as m
                left join Data as d
                    on m.month = d.month;
            "#
        }
        DatabaseBackend::Sqlite => {
            r#"
            with Data as (
                select
                    strftime('%m', p.date_created) as month,
                    count(p.id) as count
                from
                    pipeline_runs as p
                where
                    p.state in ('running', 'finished', 'faulted')
                group by
                    month
                order by
                    month
            ),
            Months as (
                select '01' as month
                union all
                select '02' as month
                union all
                select '03' as month
                union all
                select '04' as month
                union all
                select '05' as month
                union all
                select '06' as month
                union all
                select '07' as month
                union all
                select '08' as month
                union all
                select '09' as month
                union all
                select '10' as month
                union all
                select '11' as month
                union all
                select '12' as month
            )
            select
                cast(m.month as bigint) as month,
                case when d.count is null then 0 else d.count end as count
            from
                Months as m
                left join Data as d
                    on m.month = d.month;
            "#
        }
    };
    PipelineRunsPerMonth::find_by_statement(Statement::from_sql_and_values(
        conn.get_database_backend(),
        query,
        [],
    ))
    .all(conn)
    .await
    .inspect(|_| debug!("got the count of pipeline runs per month successfully"))
    .map_err(|e| {
        error!("could not get the count of pipeline runs per month due to: {e}");
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
