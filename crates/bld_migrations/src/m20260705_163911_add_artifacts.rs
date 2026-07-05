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
                    .table(Artifacts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Artifacts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Artifacts::RunId).string().not_null())
                    .col(ColumnDef::new(Artifacts::Name).string().not_null())
                    .col(ColumnDef::new(Artifacts::Path).string().not_null())
                    .col(
                        ColumnDef::new(Artifacts::DateCreated)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Artifacts::DateUpdated).date_time())
                    .col(
                        ColumnDef::new(Artifacts::DateExpires)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(Artifacts::Table)
                            .from_col(Artifacts::RunId)
                            .to_tbl(PipelineRuns::Table)
                            .to_col(PipelineRuns::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Artifacts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Artifacts {
    Table,
    Id,
    RunId,
    Name,
    Path,
    DateCreated,
    DateUpdated,
    DateExpires,
}
