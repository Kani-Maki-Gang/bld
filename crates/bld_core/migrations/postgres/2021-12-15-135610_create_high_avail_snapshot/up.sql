-- Your SQL goes here
create table ha_snapshot (
  id bigint primary key not null,
  term bigint not null,
  data bytea not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create or replace function ha_snapshot_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_snapshot_after_update
after update on ha_snapshot
for each row execute procedure ha_snapshot_after_update();

create table ha_members (
  id bigint primary key not null,
  snapshot_id bigint not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(snapshot_id) references ha_snapshot(id)
);

create or replace function ha_members_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_members_after_update
after update on ha_members
for each row execute procedure ha_members_after_update();

create table ha_members_after_consensus (
  id bigint primary key not null,
  snapshot_id bigint not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(snapshot_id) references ha_snapshot(id)
);

create or replace function ha_members_after_consensus_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_ha_members_after_consensus_after_update
after update on ha_members_after_consensus
for each row execute procedure ha_members_after_consensus_after_update();