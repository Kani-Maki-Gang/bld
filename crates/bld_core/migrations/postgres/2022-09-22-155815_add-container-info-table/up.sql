-- Your SQL goes here
create table pipeline_run_containers (
  id text primary key not null,
  run_id text not null,
  container_id text not null,
  state text not null,
  date_created text default current_timestamp not null,
  foreign key(run_id) references pipeline_runs(id)
);
