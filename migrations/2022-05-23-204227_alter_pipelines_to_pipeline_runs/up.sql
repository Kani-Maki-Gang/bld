-- Your SQL goes here
create table pipeline_runs (
    id text primary key not null,
    name text not null,
    running boolean not null,
    user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text
);

create trigger pipeline_runs_after_update
    after update on pipeline_runs
begin
    update pipeline_runs set end_date_time = current_timestamp where id = new.id and running = false;
end;

insert into pipeline_runs (
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
  pipelines;

drop table pipelines;
