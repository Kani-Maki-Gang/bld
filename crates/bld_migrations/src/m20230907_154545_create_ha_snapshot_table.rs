use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HighAvailabilitySnapshot::Table)
                    .col(
                        ColumnDef::new(HighAvailabilitySnapshot::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilitySnapshot::Term)
                            .integer()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(HighAvailabilitySnapshot::Data)
                            .binary()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(HighAvailabilitySnapshot::DateCreated)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilitySnapshot::DateUpdated)
                            .timestamp()
                            .not_null(),
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
                    .table(HighAvailabilitySnapshot::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum HighAvailabilitySnapshot {
    Table,
    Id,
    Term,
    Data,
    DateCreated,
    DateUpdated,
}
