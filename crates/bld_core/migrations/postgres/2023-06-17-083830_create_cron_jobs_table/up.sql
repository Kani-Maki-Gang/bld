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

create or replace function cron_jobs_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_cron_jobs_after_update
after update on cron_jobs
for each row execute procedure cron_jobs_after_update();

create table if not exists cron_job_variables (
    id text primary key not null,
    name text not null,
    value text not null,
    cron_job_id text not null,
    date_created text default current_timestamp not null,
    date_updated text,
    foreign key(cron_job_id) references cron_jobs(id)
);

create or replace function cron_job_variables_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_cron_job_variables_after_update
after update on cron_job_variables
for each row execute procedure cron_job_variables_after_update();

create table if not exists cron_job_environment_variables (
    id text primary key not null,
    name text not null,
    value text not null,
    cron_job_id text not null,
    date_created text not null,
    date_updated text,
    foreign key(cron_job_id) references cron_jobs(id)
);

create or replace function cron_job_environment_variables_after_update() returns trigger language plpgsql as $$
begin
    new.end_date_time = current_timestamp;
    return new;
end;
$$;

create trigger tg_cron_job_environment_variables_after_update
after update on cron_job_environment_variables
for each row execute procedure cron_job_environment_variables_after_update();
