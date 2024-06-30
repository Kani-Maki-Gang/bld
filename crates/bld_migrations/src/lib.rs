pub use sea_orm_migration::prelude::*;

mod m20230907_121524_create_ha_state_machine_table;
mod m20230907_134943_create_ha_client_serial_responses_table;
mod m20230907_150533_create_ha_client_status_table;
mod m20230907_152458_create_ha_hard_state_table;
mod m20230907_153905_create_ha_log_table;
mod m20230907_154545_create_ha_snapshot_table;
mod m20230907_155136_create_ha_members_table;
mod m20230907_155841_create_ha_members_after_consensus_table;
mod m20230907_181924_create_pipeline_table;
mod m20230907_182138_create_pipeline_runs_table;
mod m20230907_182418_create_pipeline_run_containers_table;
mod m20230907_190009_create_cron_jobs_table;
mod m20230907_190403_create_cron_job_variables_table;
mod m20230907_190709_create_cron_job_environment_variables_table;
mod m20240630_162930_login_attempts;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230907_121524_create_ha_state_machine_table::Migration),
            Box::new(m20230907_134943_create_ha_client_serial_responses_table::Migration),
            Box::new(m20230907_150533_create_ha_client_status_table::Migration),
            Box::new(m20230907_152458_create_ha_hard_state_table::Migration),
            Box::new(m20230907_153905_create_ha_log_table::Migration),
            Box::new(m20230907_154545_create_ha_snapshot_table::Migration),
            Box::new(m20230907_155136_create_ha_members_table::Migration),
            Box::new(m20230907_155841_create_ha_members_after_consensus_table::Migration),
            Box::new(m20230907_181924_create_pipeline_table::Migration),
            Box::new(m20230907_182138_create_pipeline_runs_table::Migration),
            Box::new(m20230907_182418_create_pipeline_run_containers_table::Migration),
            Box::new(m20230907_190009_create_cron_jobs_table::Migration),
            Box::new(m20230907_190403_create_cron_job_variables_table::Migration),
            Box::new(m20230907_190709_create_cron_job_environment_variables_table::Migration),
            Box::new(m20240630_162930_login_attempts::Migration),
        ]
    }
}
