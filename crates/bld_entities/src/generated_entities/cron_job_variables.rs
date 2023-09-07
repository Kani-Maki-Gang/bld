//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "cron_job_variables")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
    pub date_created: DateTime,
    pub date_updated: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::cron_jobs::Entity",
        from = "Column::CronJobId",
        to = "super::cron_jobs::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    CronJobs,
}

impl Related<super::cron_jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CronJobs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
