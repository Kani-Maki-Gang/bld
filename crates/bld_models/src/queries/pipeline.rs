use crate::{
    dtos::FileType,
    generated::pipeline::{self, Entity as PipelineEntity},
};
use anyhow::{Result, anyhow};
use bld_migrations::Expr;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, ModelTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use tracing::{debug, error};

use super::cron_jobs;

pub use crate::generated::pipeline::Model as Pipeline;

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
        .inspect(|_| {
            debug!("loaded all pipelines successfully");
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
        .inspect(|_| {
            debug!("loaded pipeline successfully");
        })
        .map_err(|e| {
            error!("could not load pipeline due to {e}");
            anyhow!(e)
        })?;

    model.ok_or_else(|| {
        error!("couldn't load pipeline due to not found");
        anyhow!("file not found")
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
            anyhow!("file not found")
        })
        .inspect(|_| {
            debug!("loaded pipeline successfully");
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

pub async fn update_file_type<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    pip_name: &str,
    file_type: FileType,
) -> Result<()> {
    debug!("updating pipeline with name: {pip_name} with file type: {file_type:?}");
    PipelineEntity::update_many()
        .col_expr(pipeline::Column::FileType, Expr::value(file_type.as_str()))
        .filter(pipeline::Column::Name.eq(pip_name))
        .exec(conn)
        .await
        .map(|_| debug!("pipeline file type updated successfully"))
        .map_err(|e| {
            error!("could not update pipeline file type due to {e}");
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
        date_created: Set(Utc::now().naive_utc()),
        file_type: Set(FileType::Pipeline.as_str().to_owned()),
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
        })?;
    txn.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{InsertPipeline, insert, select_by_name, update_file_type};
    use crate::dtos::FileType;
    use bld_migrations::{Migrator, MigratorTrait};
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

    async fn connection() -> DatabaseConnection {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&conn, None).await.unwrap();
        conn
    }

    #[tokio::test]
    async fn existing_entries_default_to_pipeline_after_migration() {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        let pending = Migrator::get_pending_migrations(&conn).await.unwrap().len();
        Migrator::up(&conn, Some(pending as u32 - 1)).await.unwrap();
        conn.execute_unprepared(
            "insert into pipeline (id, name, date_created) values ('1', 'old.yaml', '2026-01-01 00:00:00')",
        )
        .await
        .unwrap();
        Migrator::up(&conn, None).await.unwrap();

        let model = select_by_name(&conn, "old.yaml").await.unwrap();
        assert_eq!(FileType::from(model.file_type.as_str()), FileType::Pipeline);
    }

    #[tokio::test]
    async fn file_type_is_updated_by_name() {
        let conn = connection().await;
        let model = InsertPipeline {
            id: "1".to_owned(),
            name: "action.yaml".to_owned(),
        };
        insert(&conn, model).await.unwrap();
        assert_eq!(
            select_by_name(&conn, "action.yaml")
                .await
                .unwrap()
                .file_type,
            "pipeline"
        );

        update_file_type(&conn, "action.yaml", FileType::Action)
            .await
            .unwrap();

        assert_eq!(
            select_by_name(&conn, "action.yaml")
                .await
                .unwrap()
                .file_type,
            "action"
        );
    }
}
