-- Your SQL goes here
create table ha_hard_state (
  id integer primary key autoincrement not null,
  current_term integer not null,
  voted_for integer,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create trigger ha_hard_state_after_update
  after update on ha_hard_state
begin
  update ha_hard_state set date_updated = current_timestamp where id = new.id;
end;
