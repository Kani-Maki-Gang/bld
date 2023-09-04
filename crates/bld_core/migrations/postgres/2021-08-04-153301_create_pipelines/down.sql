-- This file should undo anything in `up.sql`
drop table pipelines;
drop trigger if exists tg_pipelines_after_update on pipelines;
drop function pipelines_after_update();
