-- Your SQL goes here
create table if not exists cron_jobs (
    id text primary key not null,
    pipeline_id text not null,
    schedule text not null,
    is_default boolean not null,
    date_created text default current_timestamp not null,
    date_updated text,
    foreign key(pipeline_id) references pipeline(id)
);

create trigger cron_job_after_update
    after update on cron_jobs
begin
    update cron_jobs set date_updated = current_timestamp where id = new.id;
end;

create table if not exists cron_job_variables (
    id text primary key not null,
    name text not null,
    value text not null,
    cron_job_id text not null,
    foreign key(cron_job_id) references cron_jobs(id)
);

create table if not exists cron_job_environment_variables (
    id text primary key not null,
    name text not null,
    value text not null,
    cron_job_id text not null,
    foreign key(cron_job_id) references cron_jobs(id)
);
