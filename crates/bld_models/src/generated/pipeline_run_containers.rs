//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "pipeline_run_containers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub run_id: String,
    pub container_id: String,
    pub state: String,
    pub date_created: DateTime,
    pub date_updated: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::pipeline_runs::Entity",
        from = "Column::RunId",
        to = "super::pipeline_runs::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    PipelineRuns,
}

impl Related<super::pipeline_runs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PipelineRuns.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
