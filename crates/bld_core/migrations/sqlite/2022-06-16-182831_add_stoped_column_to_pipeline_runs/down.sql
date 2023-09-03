-- This file should undo anything in `up.sql`
-- The below are done for versions of sqlite that dont support drop column.
create table pipeline_runs_temp (
    id text primary key not null,
    name text not null,
    running boolean not null,
    app_user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text,
    stopped boolean
);

insert into pipeline_runs_temp (
  id,
  name,
  running,
  app_user,
  start_date_time,
  end_date_time,
  stopped
)
select
  id,
  name,
  running,
  app_user,
  start_date_time,
  end_date_time,
  stopped
from
  pipeline_runs;

drop table pipeline_runs;

create table pipeline_runs (
    id text primary key not null,
    name text not null,
    running boolean not null,
    app_user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text
);

create trigger pipeline_runs_after_update
    after update on pipeline_runs
begin
    update pipeline_runs set end_date_time = current_timestamp where id = new.id and running = false;
end;
