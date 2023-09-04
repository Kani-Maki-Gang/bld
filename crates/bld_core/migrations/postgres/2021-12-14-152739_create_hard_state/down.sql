-- This file should undo anything in `up.sql`
drop table ha_hard_state;
drop trigger if exists tg_ha_hard_state_after_update on ha_hard_state;
drop function ha_hard_state_after_update();
