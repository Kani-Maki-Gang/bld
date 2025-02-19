//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "login_attempts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub csrf_token: String,
    pub nonce: String,
    pub pkce_verifier: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub status: String,
    pub date_created: DateTime,
    pub date_updated: Option<DateTime>,
    pub date_expires: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
