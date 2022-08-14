-- This file should undo anything in `up.sql`
alter table pipeline_runs rename to pipeline_runs_new;

drop trigger if exists pipeline_runs_after_update;

create table pipeline_runs (
    id text primary key not null,
    name text not null,
    running boolean not null,
    user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text,
    stopped boolean
);

insert into pipeline_runs (
  id,
  name,
  running,
  user,
  start_date_time,
  end_date_time,
  stopped
)
select
  id,
  name,
  case (state)
    when 'running' then true
    else false
  end as state,
  user,
  start_date_time,
  end_date_time,
  stopped
from
  pipeline_runs_new;

create trigger pipeline_runs_after_update
    after update on pipeline_runs
begin
    update pipeline_runs set end_date_time = current_timestamp where id = new.id and running = false;
end;

drop table pipeline_runs_new;
