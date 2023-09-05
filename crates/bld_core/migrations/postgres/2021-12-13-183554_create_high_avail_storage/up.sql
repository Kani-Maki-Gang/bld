-- Your SQL goes here
create table ha_state_machine (
  id integer generated always as identity primary key not null,
  last_applied_log integer not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create or replace function ha_state_machine_after_update() returns trigger language plpgsql as $$
begin
    new.date_updated = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_state_machine_after_update
after update on ha_state_machine
for each row execute procedure ha_state_machine_after_update();

create table ha_client_serial_responses (
  id integer primary key not null,
  state_machine_id integer not null,
  serial integer not null,
  response text,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(state_machine_id) references ha_state_machine(id)
);

create or replace function ha_client_serial_responses_after_update() returns trigger language plpgsql as $$
begin
    new.date_updated = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_client_serial_responses_after_update
after update on ha_client_serial_responses
for each row execute procedure ha_client_serial_responses_after_update();

create table ha_client_status (
  id integer primary key not null,
  state_machine_id integer not null,
  status text not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(state_machine_id) references ha_state_machine(id)
);

create or replace function ha_client_status_after_update() returns trigger language plpgsql as $$
begin
    new.date_updated = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_client_status_after_update
after update on ha_client_status
for each row execute procedure ha_client_status_after_update();
