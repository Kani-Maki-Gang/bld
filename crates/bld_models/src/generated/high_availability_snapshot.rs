//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "high_availability_snapshot")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    pub term: i32,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub data: Vec<u8>,
    pub date_created: DateTime,
    pub date_updated: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::high_availability_members::Entity")]
    HighAvailabilityMembers,
    #[sea_orm(has_many = "super::high_availability_members_after_consensus::Entity")]
    HighAvailabilityMembersAfterConsensus,
}

impl Related<super::high_availability_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HighAvailabilityMembers.def()
    }
}

impl Related<super::high_availability_members_after_consensus::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HighAvailabilityMembersAfterConsensus.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
