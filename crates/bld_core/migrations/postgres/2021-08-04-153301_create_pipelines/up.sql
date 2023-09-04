-- Your SQL goes here
create table pipelines (
    id text primary key not null,
    name text not null,
    running boolean not null,
    app_user text not null,
    start_date_time text default current_timestamp not null,
    end_date_time text
);

create or replace function pipelines_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_pipelines_after_update
after update on pipelines
for each row execute procedure pipelines_after_update();
