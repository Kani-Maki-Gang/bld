-- This file should undo anything in `up.sql`
drop table cron_job_environment_variables;
drop trigger if exists tg_cron_job_environment_variables_after_update on cron_job_environment_variables;
drop function cron_job_environment_variables_after_update();
drop table cron_job_variables;
drop trigger if exists tg_cron_job_variables_after_update on cron_job_variables;
drop function cron_job_variables_after_update();
drop table cron_jobs;
drop trigger if exists tg_cron_jobs_after_update on cron_jobs;
drop function cron_jobs_after_update();

