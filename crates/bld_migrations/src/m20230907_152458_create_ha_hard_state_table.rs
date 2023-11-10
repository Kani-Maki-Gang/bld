use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HighAvailabilityHardState::Table)
                    .col(
                        ColumnDef::new(HighAvailabilityHardState::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityHardState::CurrentTerm)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HighAvailabilityHardState::VotedFor).integer())
                    .col(
                        ColumnDef::new(HighAvailabilityHardState::DateCreated)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HighAvailabilityHardState::DateUpdated).date_time())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(HighAvailabilityHardState::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum HighAvailabilityHardState {
    Table,
    Id,
    CurrentTerm,
    VotedFor,
    DateCreated,
    DateUpdated,
}
