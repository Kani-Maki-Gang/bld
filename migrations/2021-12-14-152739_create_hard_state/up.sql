-- Your SQL goes here
create table ha_hard_state (
  id nvarchar(50) primary key not null,
  current_term int not null,
  voted_for int,
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null
)
