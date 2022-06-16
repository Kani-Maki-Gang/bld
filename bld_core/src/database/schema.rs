table! {
    ha_client_serial_responses (id) {
        id -> Integer,
        state_machine_id -> Integer,
        serial -> Integer,
        response -> Nullable<Text>,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_client_status (id) {
        id -> Integer,
        state_machine_id -> Integer,
        status -> Text,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_hard_state (id) {
        id -> Integer,
        current_term -> Integer,
        voted_for -> Nullable<Integer>,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_log (id) {
        id -> Integer,
        term -> Integer,
        payload_type -> Text,
        payload -> Text,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_members (id) {
        id -> Integer,
        snapshot_id -> Integer,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_members_after_consensus (id) {
        id -> Integer,
        snapshot_id -> Integer,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_snapshot (id) {
        id -> Integer,
        term -> Integer,
        data -> Binary,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_state_machine (id) {
        id -> Integer,
        last_applied_log -> Integer,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    pipeline (id) {
        id -> Text,
        name -> Text,
        date_created -> Text,
    }
}

table! {
    pipeline_runs (id) {
        id -> Text,
        name -> Text,
        running -> Bool,
        user -> Text,
        start_date_time -> Text,
        end_date_time -> Nullable<Text>,
        stopped -> Nullable<Bool>,
    }
}

joinable!(ha_client_serial_responses -> ha_state_machine (state_machine_id));
joinable!(ha_client_status -> ha_state_machine (state_machine_id));
joinable!(ha_members -> ha_snapshot (snapshot_id));
joinable!(ha_members_after_consensus -> ha_snapshot (snapshot_id));

allow_tables_to_appear_in_same_query!(
    ha_client_serial_responses,
    ha_client_status,
    ha_hard_state,
    ha_log,
    ha_members,
    ha_members_after_consensus,
    ha_snapshot,
    ha_state_machine,
    pipeline,
    pipeline_runs,
);
