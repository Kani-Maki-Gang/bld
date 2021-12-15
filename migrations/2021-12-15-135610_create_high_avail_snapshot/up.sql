-- Your SQL goes here
create table ha_snapshot (
  id int primary key not null,
  term int not null,
  data blob not null,
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null
);

create table ha_members ( 
  id int primary key not null,
  snapshot_id int not null,
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null,
  foreign key(snapshot_id) references ha_snapshot(id)
);

create table ha_members_after_consensus (
  id int primary key not null,
  snapshot_id int not null,
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null,
  foreign key(snapshot_id) references ha_snapshot(id)
)
