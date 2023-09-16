use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HighAvailabilityLog::Table)
                    .col(
                        ColumnDef::new(HighAvailabilityLog::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityLog::Term)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityLog::PayloadType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityLog::Payload)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HighAvailabilityLog::DateCreated)
                            .timestamp()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp))
                            .not_null(),
                    )
                    .col(ColumnDef::new(HighAvailabilityLog::DateUpdated).timestamp())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HighAvailabilityLog::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum HighAvailabilityLog {
    Table,
    Id,
    Term,
    PayloadType,
    Payload,
    DateCreated,
    DateUpdated,
}
