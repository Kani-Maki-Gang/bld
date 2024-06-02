use super::{
    edit::{CronJobsEdit, SaveCronJob},
    helpers::{get_pipeline, hash_map_strings},
};
use anyhow::{bail, Result};
use bld_models::dtos::{AddJobRequest, CronJobResponse};
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

async fn insert(data: AddJobRequest) -> Result<()> {
    let resp = Client::builder()
        .build()?
        .post("http://localhost:6080/v1/cron")
        .json(&data)
        .send()
        .await?;

    if resp.status().is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let msg = format!("Request failed with status: {:?}", resp);
        logging::console_error(&msg);
        bail!(msg)
    }
}

#[component]
pub fn CronJobInsert() -> impl IntoView {
    let params = use_query_map();
    let name = move || params.with(|p| p.get("name").cloned());
    let (save, set_save) = create_signal(None);

    let cron = move || {
        if let Some(name) = name() {
            let job = CronJobResponse {
                pipeline: name.clone(),
                ..Default::default()
            };
            Some(job)
        } else {
            None
        }
    };

    let data = create_resource(
        move || name(),
        |name| async move {
            get_pipeline(name)
                .await
                .map_err(|e| logging::console_error(&e.to_string()))
        },
    );

    let save_action = create_action(|args: &(Option<String>, SaveCronJob)| {
        let (name, (schedule, vars, env)) = args;
        let name = name.clone();
        let schedule = schedule.clone();
        let vars = hash_map_strings(vars.clone());
        let env = hash_map_strings(env.clone());

        async move {
            let Some(name) = name else {
                return;
            };
            let data = AddJobRequest::new(schedule, name.to_string(), Some(vars), Some(env), false);
            let _ = insert(data).await;
        }
    });

    create_effect(move |_| {
        if let Some(data) = save.get() {
            save_action.dispatch((name(), data));
        }
    });

    view! {
        <Show
            when=move || data.get().is_some()
            fallback=|| view! {
                <div class="text-2xl">
                    "Loading..."
                </div>
            }>
            <CronJobsEdit cron=move || cron() pipeline=move || data.get().unwrap().ok() save=set_save />
        </Show>
    }
}
