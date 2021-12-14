-- Your SQL goes here
create table ha_hard_state (
  id nvarchar(50) primary key not null,
  current_term numeric not null,
  voted_for numeric
)
