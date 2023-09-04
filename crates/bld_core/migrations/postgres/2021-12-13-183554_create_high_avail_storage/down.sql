-- This file should undo anything in `up.sql`
drop table ha_client_serial_responses;
drop trigger if exists tg_ha_client_serial_responses_after_update on ha_client_serial_responses;
drop function ha_client_serial_responses_after_update();
drop table ha_client_status;
drop trigger if exists tg_ha_client_status_after_update on ha_client_status;
drop function ha_client_status_after_update();
drop table ha_state_machine;
drop trigger if exists tg_ha_state_machine_after_update on ha_state_machine;
drop function ha_state_machine_after_update();
