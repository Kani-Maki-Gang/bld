use sea_orm_migration::prelude::*;

use crate::m20230907_181924_create_pipeline_table::Pipeline;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Pipeline::Table)
                    .add_column(
                        ColumnDef::new(FileType::FileType)
                            .string()
                            .not_null()
                            .default("pipeline"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Pipeline::Table)
                    .drop_column(FileType::FileType)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum FileType {
    FileType,
}
