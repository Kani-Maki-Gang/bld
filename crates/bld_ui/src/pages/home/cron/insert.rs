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
    let (cron, set_cron) = create_signal(None);
    let (pipeline, set_pipeline) = create_signal(None);
    let (save, set_save) = create_signal(None);

    create_resource(
        move || (name(), set_pipeline, set_cron),
        |(name, set_pipeline, set_cron)| async move {
            let Some(name) = name else {
                return;
            };

            let job = CronJobResponse {
                pipeline: name.clone(),
                ..Default::default()
            };

            let pipeline_resp = get_pipeline(name.to_string())
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .ok();

            if let Some(pipeline_resp) = pipeline_resp {
                set_pipeline.set(pipeline_resp);
                set_cron.set(Some(job));
            }
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
        <CronJobsEdit cron=cron pipeline=pipeline save=set_save />
    }
}
