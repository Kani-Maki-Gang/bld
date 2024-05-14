mod details;
mod schedule;

use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{CronJobResponse, JobFiltersParams};
use crate::pages::home::{PipelineVariable, RunPipelineVariables};
use details::CronJobsEditDetails;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use schedule::CronJobsEditSchedule;
use std::collections::HashMap;
use reqwest::Client;

fn into_pipeline_variables(items: HashMap<String, String>) -> Vec<PipelineVariable> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, (k, v))| PipelineVariable {
            id: i.to_string(),
            name: k,
            value: create_rw_signal(v),
        })
        .collect()
}

async fn get_cron(id: Option<String>) -> Result<CronJobResponse> {
    let id = id.ok_or_else(|| anyhow!("No id provided"))?;
    let params = JobFiltersParams {
        id: Some(id),
        ..Default::default()
    };

    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/cron")
        .query(&params)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        let data: Vec<CronJobResponse> = serde_json::from_str(&body)?;
        data.into_iter().next().ok_or_else(|| anyhow!("No data found"))
    } else {
        bail!("Request failed with status: {}", res.status())
    }
}

#[component]
pub fn CronJobsEdit() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let (cron, set_cron) = create_signal(None);
    let schedule = create_rw_signal(String::new());
    let variables = create_rw_signal(vec![]);
    let environment = create_rw_signal(vec![]);

    let _ = create_resource(
        move || (id(), set_cron),
        |(id, set_cron)| async move {
            let cron = get_cron(id)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .ok();
            set_cron.set(cron);
        },
    );

    create_effect(move |_| {
        if let Some(cron) = cron.get() {
            schedule.set(cron.schedule);
            if let Some(vars) = cron.variables {
                variables.set(into_pipeline_variables(vars));
            }
            if let Some(env) = cron.environment {
                environment.set(into_pipeline_variables(env));
            }
        }
    });

    view! {
        <Show
            when=move || cron.get().is_some()
            fallback=|| view! {
                <div class="text-2xl">
                    "Loading..."
                </div>
            }>
            <div class="flex flex-col gap-4">
                <CronJobsEditDetails job=move || cron.get().unwrap() />
                <CronJobsEditSchedule schedule=schedule />
                <Show
                    when=move || !variables.get().is_empty()
                    fallback=|| view! {}>
                    <RunPipelineVariables
                        title="Variables"
                        subtitle="The variables provided in the cron job run"
                        items=variables />
                </Show>
                <Show
                    when=move || !environment.get().is_empty()
                    fallback=|| view! {}>
                    <RunPipelineVariables
                        title="Environment"
                        subtitle="The environment variables provided in the cron job run"
                        items=environment />
                </Show>
            </div>
        </Show>
    }
}
