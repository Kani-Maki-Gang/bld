-- This file should undo anything in `up.sql`
alter table pipeline_runs disable trigger tg_pipeline_runs_after_update;
alter table pipeline_runs add column running boolean not null;

update
    pipeline_runs
set
    running = case (state)
        when 'running' then true
        else false
    end;

alter table pipeline_runs drop column state;
alter table pipeline_runs enable trigger tg_pipeline_runs_after_update;
