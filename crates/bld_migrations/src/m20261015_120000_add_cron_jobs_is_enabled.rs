use sea_orm_migration::prelude::*;

use crate::m20230907_190009_create_cron_jobs_table::CronJobs;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CronJobs::Table)
                    .add_column(
                        ColumnDef::new(IsEnabled::IsEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CronJobs::Table)
                    .drop_column(IsEnabled::IsEnabled)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum IsEnabled {
    IsEnabled,
}
