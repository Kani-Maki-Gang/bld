-- Your SQL goes here
create table ha_log (
  id integer primary key not null,
  term integer not null,
  payload_type text not null,
  payload text not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create or replace function ha_log_after_update() returns trigger language plpgsql as $$
begin
    new.date_updated = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_log_after_update
after update on ha_log
for each row execute procedure ha_log_after_update();
