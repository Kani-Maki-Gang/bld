use sea_orm_migration::prelude::*;

use crate::m20230907_154545_create_ha_snapshot_table::HighAvailabilitySnapshot;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HighAvailabilityMembersAfterConsensus::Table)
                    .col(
                        ColumnDef::new(HighAvailabilityMembersAfterConsensus::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityMembersAfterConsensus::SnapshotId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityMembersAfterConsensus::DateCreated)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityMembersAfterConsensus::DateUpdated)
                            .date_time(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(HighAvailabilityMembersAfterConsensus::Table)
                            .from_col(HighAvailabilityMembersAfterConsensus::SnapshotId)
                            .to_tbl(HighAvailabilitySnapshot::Table)
                            .to_col(HighAvailabilitySnapshot::Id),
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
                    .table(HighAvailabilityMembersAfterConsensus::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum HighAvailabilityMembersAfterConsensus {
    Table,
    Id,
    SnapshotId,
    DateCreated,
    DateUpdated,
}
