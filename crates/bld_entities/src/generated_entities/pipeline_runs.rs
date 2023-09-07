//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "pipeline_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub state: String,
    pub app_user: String,
    pub start_date: DateTime,
    pub end_date: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::pipeline_run_containers::Entity")]
    PipelineRunContainers,
}

impl Related<super::pipeline_run_containers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PipelineRunContainers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
