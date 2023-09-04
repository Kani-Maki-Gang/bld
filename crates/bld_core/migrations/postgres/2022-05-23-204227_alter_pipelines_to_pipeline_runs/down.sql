-- This file should undo anything in `up.sql`
alter table if exists pipeline_runs rename to pipelines;
alter trigger tg_pipeline_runs_after_update on pipelines rename to tg_pipelines_after_update;
alter function pipeline_runs_after_update() rename to pipelines_after_update;
