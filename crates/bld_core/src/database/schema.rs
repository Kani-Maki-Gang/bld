// @generated automatically by Diesel CLI.

diesel::table! {
    cron_job_environment_variables (id) {
        id -> Text,
        name -> Text,
        value -> Text,
        cron_job_id -> Text,
    }
}

diesel::table! {
    cron_job_variables (id) {
        id -> Text,
        name -> Text,
        value -> Text,
        cron_job_id -> Text,
    }
}

diesel::table! {
    cron_jobs (id) {
        id -> Text,
        pipeline_id -> Text,
        schedule -> Text,
        is_default -> Bool,
        date_created -> Text,
        date_updated -> Nullable<Text>,
    }
}

diesel::table! {
    ha_client_serial_responses (id) {
        id -> Integer,
        state_machine_id -> Integer,
        serial -> Integer,
        response -> Nullable<Text>,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_client_status (id) {
        id -> Integer,
        state_machine_id -> Integer,
        status -> Text,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_hard_state (id) {
        id -> Integer,
        current_term -> Integer,
        voted_for -> Nullable<Integer>,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_log (id) {
        id -> Integer,
        term -> Integer,
        payload_type -> Text,
        payload -> Text,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_members (id) {
        id -> Integer,
        snapshot_id -> Integer,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_members_after_consensus (id) {
        id -> Integer,
        snapshot_id -> Integer,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_snapshot (id) {
        id -> Integer,
        term -> Integer,
        data -> Binary,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    ha_state_machine (id) {
        id -> Integer,
        last_applied_log -> Integer,
        date_created -> Text,
        date_updated -> Text,
    }
}

diesel::table! {
    pipeline (id) {
        id -> Text,
        name -> Text,
        date_created -> Text,
    }
}

diesel::table! {
    pipeline_run_containers (id) {
        id -> Text,
        run_id -> Text,
        container_id -> Text,
        state -> Text,
        date_created -> Text,
    }
}

diesel::table! {
    pipeline_runs (id) {
        id -> Text,
        name -> Text,
        state -> Text,
        user -> Text,
        start_date_time -> Text,
        end_date_time -> Nullable<Text>,
    }
}

diesel::joinable!(cron_job_environment_variables -> cron_jobs (cron_job_id));
diesel::joinable!(cron_job_variables -> cron_jobs (cron_job_id));
diesel::joinable!(cron_jobs -> pipeline (pipeline_id));
diesel::joinable!(ha_client_serial_responses -> ha_state_machine (state_machine_id));
diesel::joinable!(ha_client_status -> ha_state_machine (state_machine_id));
diesel::joinable!(ha_members -> ha_snapshot (snapshot_id));
diesel::joinable!(ha_members_after_consensus -> ha_snapshot (snapshot_id));
diesel::joinable!(pipeline_run_containers -> pipeline_runs (run_id));

diesel::allow_tables_to_appear_in_same_query!(
    cron_job_environment_variables,
    cron_job_variables,
    cron_jobs,
    ha_client_serial_responses,
    ha_client_status,
    ha_hard_state,
    ha_log,
    ha_members,
    ha_members_after_consensus,
    ha_snapshot,
    ha_state_machine,
    pipeline,
    pipeline_run_containers,
    pipeline_runs,
);
