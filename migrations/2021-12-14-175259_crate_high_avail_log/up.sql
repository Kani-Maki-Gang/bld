-- Your SQL goes here
create table ha_log (
  id numeric primary key not null,
  term numeric not null,
  idx numeric not null,
  payload_type nvarchar(100) not null,
  payload nvarchar(250) not null
)
