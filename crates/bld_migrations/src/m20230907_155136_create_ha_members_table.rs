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
                    .table(HighAvailabilityMembers::Table)
                    .col(
                        ColumnDef::new(HighAvailabilityMembers::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityMembers::SnapshotId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityMembers::DateCreated)
                            .date_time()
                            .default(SimpleExpr::Keyword(Keyword::CurrentDate))
                            .not_null(),
                    )
                    .col(ColumnDef::new(HighAvailabilityMembers::DateUpdated).date_time())
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(HighAvailabilityMembers::Table)
                            .from_col(HighAvailabilityMembers::SnapshotId)
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
                    .table(HighAvailabilityMembers::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum HighAvailabilityMembers {
    Table,
    Id,
    SnapshotId,
    DateCreated,
    DateUpdated,
}
