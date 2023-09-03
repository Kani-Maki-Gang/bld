-- Your SQL goes here
alter table pipeline_runs rename to pipeline_runs_old;

drop trigger if exists pipeline_runs_after_update;

create table pipeline_runs (
    id text primary key not null,
    name text not null,
    state text not null,
    app_user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text,
    stopped boolean
);

insert into pipeline_runs (
  id,
  name,
  state,
  app_user,
  start_date_time,
  end_date_time,
  stopped
)
select
  id,
  name,
  case (running)
    when true then 'running'
    else 'finished'
  end as state,
  app_user,
  start_date_time,
  end_date_time,
  stopped
from
  pipeline_runs_old;

create trigger pipeline_runs_after_update
    after update on pipeline_runs
begin
    update pipeline_runs set end_date_time = current_timestamp where id = new.id and state = 'finished';
end;

drop table pipeline_runs_old;
