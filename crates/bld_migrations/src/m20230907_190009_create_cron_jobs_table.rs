use sea_orm_migration::prelude::*;

use crate::m20230907_181924_create_pipeline_table::Pipeline;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CronJobs::Table)
                    .col(
                        ColumnDef::new(CronJobs::Id)
                            .string()
                            .primary_key()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(CronJobs::PipelineId)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(CronJobs::Schedule)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(CronJobs::IsDefault)
                            .boolean()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(CronJobs::DateCreated)
                            .timestamp()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(CronJobs::DateUpdated)
                            .timestamp()
                            .not_null()
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(CronJobs::Table)
                            .from_col(CronJobs::PipelineId)
                            .to_tbl(Pipeline::Table)
                            .to_col(Pipeline::Id)
                    )
                    .to_owned()
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(CronJobs::Table)
                    .to_owned()
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum CronJobs {
    Table,
    Id,
    PipelineId,
    Schedule,
    IsDefault,
    DateCreated,
    DateUpdated,
}
