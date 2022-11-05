-- Your SQL goes here
create table ha_state_machine (
  id integer primary key autoincrement not null,
  last_applied_log integer not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create trigger ha_state_machine_after_update
  after update on ha_state_machine
begin
  update ha_state_machine set date_updated = current_timestamp where id = new.id;
end;

create table ha_client_serial_responses (
  id integer primary key not null,
  state_machine_id integer not null,
  serial integer not null,
  response text,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(state_machine_id) references ha_state_machine(id)
);

create trigger ha_client_serial_responses_after_update
  after update on ha_client_serial_responses
begin
  update ha_client_serial_responses set date_updated = current_timestamp where id = new.id;
end;

create table ha_client_status (
  id integer primary key not null,
  state_machine_id integer not null,
  status text not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(state_machine_id) references ha_state_machine(id)
);

create trigger ha_client_status
  after update on ha_client_status
begin
  update ha_client_status set date_updated = current_timestamp where id = new.id;
end;
