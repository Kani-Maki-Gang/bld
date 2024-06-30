use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LoginAttempts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LoginAttempts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LoginAttempts::CsrfToken).string().not_null())
                    .col(ColumnDef::new(LoginAttempts::Nonce).string().not_null())
                    .col(
                        ColumnDef::new(LoginAttempts::PkceVerifier)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(LoginAttempts::AccessToken).string())
                    .col(ColumnDef::new(LoginAttempts::RefreshToken).string())
                    .col(ColumnDef::new(LoginAttempts::Status).string().not_null())
                    .col(
                        ColumnDef::new(LoginAttempts::DateCreated)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(LoginAttempts::DateUpdated).date_time())
                    .col(
                        ColumnDef::new(LoginAttempts::DateExpires)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LoginAttempts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LoginAttempts {
    Table,
    Id,
    CsrfToken,
    Nonce,
    PkceVerifier,
    AccessToken,
    RefreshToken,
    Status,
    DateCreated,
    DateUpdated,
    DateExpires,
}
