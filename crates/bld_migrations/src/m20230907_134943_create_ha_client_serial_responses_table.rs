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
                    .table(HighAvailabilityClientSerialResponses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HighAvailabilityClientSerialResponses::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityClientSerialResponses::StateMachineId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityClientSerialResponses::Serial)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HighAvailabilityClientSerialResponses::Response).string())
                    .col(
                        ColumnDef::new(HighAvailabilityClientSerialResponses::DateCreated)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityClientSerialResponses::DateUpdated)
                            .date_time(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(HighAvailabilityClientSerialResponses::Table)
                            .from_col(HighAvailabilityClientSerialResponses::StateMachineId)
                            .to_tbl(HighAvailabilityStateMachine::Table)
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
                    .table(HighAvailabilityClientSerialResponses::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum HighAvailabilityClientSerialResponses {
    Table,
    Id,
    StateMachineId,
    Serial,
    Response,
    DateCreated,
    DateUpdated,
}
