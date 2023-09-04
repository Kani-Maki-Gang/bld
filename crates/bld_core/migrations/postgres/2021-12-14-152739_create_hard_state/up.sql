-- Your SQL goes here
create table ha_hard_state (
  id bigint generated always as identity primary key not null,
  current_term bigint not null,
  voted_for bigint,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create or replace function ha_hard_state_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_hard_state_after_update
after update on ha_hard_state
for each row execute procedure ha_hard_state_after_update();
