-- Your SQL goes here
create table ha_state_machine (
  id nvarchar(50) primary key not null,
  last_applied_log int not null,
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null
);

create table ha_client_serial_responses (
  id nvarchar(50) primary key not null,
  state_machine_id nvarchar(50) not null,
  serial nvarchar(100) not null,
  previous nvarchar(50),
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null,
  foreign key(state_machine_id) references ha_state_machine(id)
);

create table ha_client_status (
  id nvarchar(50) primary key not null,
  state_machine_id nvarchar(50) not null,
  status nvarchar(100),
  date_created nvarchar(100) not null,
  date_updated nvarchar(100) not null,
  foreign key(state_machine_id) references ha_state_machine(id)
)
