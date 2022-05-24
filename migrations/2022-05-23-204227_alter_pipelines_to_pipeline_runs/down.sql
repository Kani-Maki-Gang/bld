-- This file should undo anything in `up.sql`
create table pipelines (
    id text primary key not null,
    name text not null,
    running boolean not null,
    user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text
);

drop trigger pipelines_after_update;

create trigger pipelines_after_update
    after update on pipelines
begin
    update pipelines set end_date_time = current_timestamp where id = new.id and running = false;
end;

insert into pipelines (
  id,
  name,
  running,
  user,
  start_date_time,
  end_date_time
)
select 
  id,
  name,
  running,
  user,
  start_date_time,
  end_date_time
from
  pipeline_runs;

drop table pipeline_runs;
