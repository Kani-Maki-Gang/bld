use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PipelineRuns::Table)
                    .col(
                        ColumnDef::new(PipelineRuns::Id)
                            .string()
                            .primary_key()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(PipelineRuns::Name)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(PipelineRuns::State)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(PipelineRuns::AppUser)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(PipelineRuns::StartDate)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PipelineRuns::EndDate)
                            .timestamp()
                            .not_null(),
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
                    .table(PipelineRuns::Table)
                    .to_owned()
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum PipelineRuns {
    Table,
    Id,
    Name,
    State,
    AppUser,
    StartDate,
    EndDate,
}
