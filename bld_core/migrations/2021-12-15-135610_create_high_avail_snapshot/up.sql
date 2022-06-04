-- Your SQL goes here
create table ha_snapshot (
  id integer primary key not null,
  term int not null,
  data blob not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null
);

create trigger ha_snapshot_after_update
  after update on ha_snapshot
begin
  update ha_snapshot set date_updated = current_timestamp where id = new.id;
end;

create table ha_members ( 
  id integer primary key not null,
  snapshot_id integer not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(snapshot_id) references ha_snapshot(id)
);

create trigger ha_members_after_update
  after update on ha_members
begin
  update ha_members set date_updated = current_timestamp where id = new.id;
end;

create table ha_members_after_consensus (
  id integer primary key not null,
  snapshot_id integer not null,
  date_created text default current_timestamp not null,
  date_updated text default current_timestamp not null,
  foreign key(snapshot_id) references ha_snapshot(id)
);

create trigger ha_members_after_consensus_after_update
  after update on ha_members_after_consensus
begin
  update ha_members_after_consensus set date_updated = current_timestamp where id = new.id;
end;


