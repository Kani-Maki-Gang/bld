-- Your SQL goes here
create table pipelines (
    id text primary key not null,
    name text not null,
    running boolean not null,
    user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text
);

create trigger pipelines_after_update
    after update on pipelines
begin
    update pipelines set end_date_time = current_timestamp where id = new.id and running = false;
end;

