-- This file should undo anything in `up.sql`
alter table pipeline_runs add column stopped boolean default false not null;
