-- Your SQL goes here
alter table if exists pipelines rename to pipeline_runs;
alter function pipelines_after_update() rename to pipeline_runs_after_update;
alter trigger tg_pipelines_after_update on pipeline_runs rename to tg_pipeline_runs_after_update;
