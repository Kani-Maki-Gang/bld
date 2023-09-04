-- This file should undo anything in `up.sql`
drop table ha_members_after_consensus;
drop trigger if exists tg_ha_members_after_consensus_after_update on ha_members_after_consensus;
drop function ha_members_after_consensus_after_update();
drop table ha_members;
drop trigger if exists tg_ha_members_after_update on ha_members;
drop function ha_members_after_update();
drop table ha_snapshot;
drop trigger if exists tg_ha_snapshot_after_update on ha_snapshot;
drop function ha_snapshot_after_update();
