use sea_orm_migration::prelude::*;

use crate::m20230907_182138_create_pipeline_runs_table::PipelineRuns;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PipelineRunContainers::Table)
                    .col(
                        ColumnDef::new(PipelineRunContainers::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PipelineRunContainers::RunId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PipelineRunContainers::ContainerId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PipelineRunContainers::State)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PipelineRunContainers::DateCreated)
                            .timestamp()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PipelineRunContainers::DateUpdated)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(PipelineRunContainers::Table)
                            .from_col(PipelineRunContainers::RunId)
                            .to_tbl(PipelineRuns::Table)
                            .to_col(PipelineRuns::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PipelineRunContainers::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum PipelineRunContainers {
    Table,
    Id,
    RunId,
    ContainerId,
    State,
    DateCreated,
    DateUpdated,
}
