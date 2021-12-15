-- Your SQL goes here
create table ha_log (
  id int primary key not null,
  term int not null,
  idx int not null,
  payload_type nvarchar(100) not null,
  payload nvarchar(250) not null,
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null
)
