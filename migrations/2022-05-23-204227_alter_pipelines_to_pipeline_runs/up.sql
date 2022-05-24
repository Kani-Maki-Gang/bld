-- Your SQL goes here
alter table pipelines rename to pipeline_runs;

alter table pipeline_runs
  add container_id text;
