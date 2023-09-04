-- Your SQL goes here
alter table pipeline_runs disable trigger tg_pipeline_runs_after_update;
alter table pipeline_runs add column state text not null;

update
    pipeline_runs
set
    state = case (running)
        when true then 'running'
        else 'finished'
    end;

alter table pipeline_runs drop column running;
alter table pipeline_runs enable trigger tg_pipeline_runs_after_update;
