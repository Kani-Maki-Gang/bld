use sea_orm_migration::prelude::*;

use crate::m20230907_190009_create_cron_jobs_table::CronJobs;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CronJobVariables::Table)
                    .col(
                        ColumnDef::new(CronJobVariables::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CronJobVariables::Name).string().not_null())
                    .col(ColumnDef::new(CronJobVariables::Value).string().not_null())
                    .col(
                        ColumnDef::new(CronJobVariables::CronJobId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CronJobVariables::DateCreated)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CronJobVariables::DateUpdated)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(CronJobVariables::Table)
                            .from_col(CronJobVariables::CronJobId)
                            .to_tbl(CronJobs::Table)
                            .to_col(CronJobs::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CronJobVariables::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CronJobVariables {
    Table,
    Id,
    Name,
    Value,
    CronJobId,
    DateCreated,
    DateUpdated,
}
