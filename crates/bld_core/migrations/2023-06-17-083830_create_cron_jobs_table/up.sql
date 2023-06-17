-- Your SQL goes here
create table if not exists cron_jobs (
    id integer primary key not null,
    pipeline_id text not null,
    schedule text not null,
    foreign key(pipeline_id) references pipeline(id)
);

create table if not exists cron_job_variables (
    id integer primary key not null,
    name text not null,
    value text not null,
    cron_job_id integer not null,
    foreign key(cron_job_id) references cron_jobs(id)
);

create table if not exists cron_job_environment_variables (
    id integer primary key not null,
    name text not null,
    value text not null,
    cron_job_id integer not null,
    foreign key(cron_job_id) references cron_jobs(id)
);
