-- This file should undo anything in `up.sql`
drop table ha_log;
drop trigger if exists tg_ha_log_after_update on ha_log;
drop function ha_log_after_update();
