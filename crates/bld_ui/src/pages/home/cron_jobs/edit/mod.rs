mod details;
mod schedule;

use crate::pages::home::{PipelineVariable, RunPipelineVariables};
use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{CronJobResponse, JobFiltersParams, PipelineInfoQueryParams};
use bld_runner::{
    pipeline::{v1, v2},
    VersionedPipeline,
};
use details::CronJobsEditDetails;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;
use schedule::CronJobsEditSchedule;
use std::collections::HashMap;

fn into_pipeline_variables(
    pipeline_items: HashMap<String, String>,
    mut cron_items: Option<HashMap<String, String>>,
) -> Vec<PipelineVariable> {
    pipeline_items
        .into_iter()
        .enumerate()
        .map(|(i, (k, v))| {
            let value = cron_items
                .as_mut()
                .and_then(|c| c.remove(&k))
                .unwrap_or_else(|| v);

            PipelineVariable {
                id: i.to_string(),
                name: k,
                value: create_rw_signal(value),
            }
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
        data.into_iter()
            .next()
            .ok_or_else(|| anyhow!("No data found"))
    } else {
        bail!("Request failed with status: {}", res.status())
    }
}

async fn get_pipeline(name: String) -> Result<Option<VersionedPipeline>> {
    let params = PipelineInfoQueryParams::Name { name };

    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/print")
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        bail!("unable to fetch pipeline")
    }
}

async fn fetch_all_data(
    id: Option<String>,
    pipeline: WriteSignal<Option<VersionedPipeline>>,
    cron: WriteSignal<Option<CronJobResponse>>,
) {
    let cron_resp = get_cron(id)
        .await
        .map_err(|e| logging::console_error(e.to_string().as_str()))
        .ok();

    if let Some(cron_resp) = &cron_resp {
        let pipeline_resp = get_pipeline(cron_resp.pipeline.clone())
            .await
            .map_err(|e| logging::console_error(e.to_string().as_str()))
            .ok();

        if let Some(pipeline_resp) = pipeline_resp {
            pipeline.set(pipeline_resp);
        }
    }

    cron.set(cron_resp);
}

#[component]
pub fn CronJobsEdit() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let (cron, set_cron) = create_signal(None);
    let (pipeline, set_pipeline) = create_signal(None);
    let schedule = create_rw_signal(String::new());
    let variables = create_rw_signal(vec![]);
    let environment = create_rw_signal(vec![]);

    create_resource(
        move || (id(), set_pipeline, set_cron),
        |(id, set_pipeline, set_cron)| async move {
            fetch_all_data(id, set_pipeline, set_cron).await;
        },
    );

    create_effect(move |_| {
        let (Some(cron), Some(pipeline)) = (cron.get(), pipeline.get()) else {
            return;
        };
        schedule.set(cron.schedule);
        let (vars, env) = pipeline.variables_and_environment();
        variables.set(into_pipeline_variables(vars, cron.variables));
        environment.set(into_pipeline_variables(env, cron.environment));
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
