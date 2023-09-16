use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HighAvailabilityStateMachine::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HighAvailabilityStateMachine::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityStateMachine::LastAppliedLog)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityStateMachine::DateCreated)
                            .timestamp()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp))
                            .not_null(),
                    )
                    .col(ColumnDef::new(HighAvailabilityStateMachine::DateUpdated).timestamp())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("idx-ha-state-machine-last-applied-log")
                    .table(HighAvailabilityStateMachine::Table)
                    .col(HighAvailabilityStateMachine::LastAppliedLog)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(HighAvailabilityStateMachine::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum HighAvailabilityStateMachine {
    Table,
    Id,
    LastAppliedLog,
    DateCreated,
    DateUpdated,
}
