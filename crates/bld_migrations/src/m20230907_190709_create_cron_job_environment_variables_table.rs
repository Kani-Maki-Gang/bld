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
                    .table(CronJobEnvironmentVariables::Table)
                    .col(
                        ColumnDef::new(CronJobEnvironmentVariables::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CronJobEnvironmentVariables::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CronJobEnvironmentVariables::Value)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CronJobEnvironmentVariables::CronJobId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CronJobEnvironmentVariables::DateCreated)
                            .date_time()
                            .default(SimpleExpr::Keyword(Keyword::CurrentDate))
                            .not_null(),
                    )
                    .col(ColumnDef::new(CronJobEnvironmentVariables::DateUpdated).date_time())
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(CronJobEnvironmentVariables::Table)
                            .from_col(CronJobEnvironmentVariables::CronJobId)
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
            .drop_table(
                Table::drop()
                    .table(CronJobEnvironmentVariables::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CronJobEnvironmentVariables {
    Table,
    Id,
    Name,
    Value,
    CronJobId,
    DateCreated,
    DateUpdated,
}
