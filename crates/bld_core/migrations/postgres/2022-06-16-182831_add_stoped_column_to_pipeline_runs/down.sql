-- This file should undo anything in `up.sql`
-- The below are done for versions of sqlite that dont support drop column.
alter table pipeline_runs drop column stopped;
