-- Your SQL goes here
create table ha_log (
  id integer primary key not null,
  term integer not null,
  payload_type text not null,
  payload text not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create trigger ha_log_after_update
  after update on ha_log
begin
  update ha_log set date_updated = current_timestamp where id = new.id;
end;
