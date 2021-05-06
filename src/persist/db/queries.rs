#![allow(dead_code)]

pub const CREATE_TABLE_PIPELINE_QUERY: &str = r"
    create table pipeline (
        id nvarchar(50) primary key not null,
        name nvarchar(250) not null,
        running boolean,
        user nvarchar(250),
        start_date_time nvarchar(100),
        end_date_time nvarchar(100)
    )
";

pub const SELECT_PIPELINES_QUERY: &str = r"
    select *
    from pipeline
";

pub const SELECT_PIPELINE_BY_ID_QUERY: &str = r"
    select * 
    from pipeline 
    where id = ? 
";

pub const SELECT_PIPELINE_BY_NAME_QUERY: &str = r"
    select *
    from pipeline
    where name = ?
";

pub const INSERT_PIPELINE_QUERY: &str = r"
    insert into pipeline
    values (?, ?, ?, ?, ?, ?)
";

pub const UPDATE_PIPELINE_QUERY: &str = r"
    update pipeline 
    set running = ?, end_date_time = ?
    where id = ?
";
