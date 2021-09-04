-- Your SQL goes here
create table pipelines (
    id nvarchar(50) primary key not null,
    name nvarchar(250) not null,
    running boolean,
    user nvarchar(250),
    start_date_time nvarchar(100),
    end_date_time nvarchar(100)
)

