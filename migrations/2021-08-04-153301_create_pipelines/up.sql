-- Your SQL goes here
create table pipelines (
    id nvarchar(50) primary key not null,
    name nvarchar(250) not null,
    running boolean not null,
    user nvarchar(250) not null,
    start_date_time nvarchar(100) not null,
    end_date_time nvarchar(100) not null
)

