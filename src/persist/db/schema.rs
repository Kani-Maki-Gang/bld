table! {
    ha_client_serial_responses (id) {
        id -> Text,
        state_machine_id -> Text,
        serial -> Text,
        previous -> Nullable<Text>,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_client_status (id) {
        id -> Text,
        state_machine_id -> Text,
        status -> Nullable<Text>,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    ha_state_machine (id) {
        id -> Text,
        last_applied_log -> Double,
        date_created -> Text,
        date_updated -> Text,
    }
}

table! {
    pipelines (id) {
        id -> Text,
        name -> Text,
        running -> Bool,
        user -> Text,
        start_date_time -> Text,
        end_date_time -> Text,
    }
}

joinable!(ha_client_serial_responses -> ha_state_machine (state_machine_id));
joinable!(ha_client_status -> ha_state_machine (state_machine_id));

allow_tables_to_appear_in_same_query!(
    ha_client_serial_responses,
    ha_client_status,
    ha_state_machine,
    pipelines,
);
