use sea_orm_migration::prelude::*;

use crate::m20230907_121524_create_ha_state_machine_table::HighAvailabilityStateMachine;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HighAvailabilityClientStatus::Table)
                    .col(
                        ColumnDef::new(HighAvailabilityClientStatus::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityClientStatus::StateMachineId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityClientStatus::DateCreated)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityClientStatus::DateUpdated)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_col(HighAvailabilityClientStatus::StateMachineId)
                            .to_col(HighAvailabilityStateMachine::Id),
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
                    .table(HighAvailabilityClientStatus::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum HighAvailabilityClientStatus {
    Table,
    Id,
    StateMachineId,
    DateCreated,
    DateUpdated,
}
