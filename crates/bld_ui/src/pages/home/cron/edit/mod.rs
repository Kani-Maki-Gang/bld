mod details;
mod schedule;

use crate::pages::home::RunPipelineVariables;
use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{
    AddJobRequest, CronJobResponse, JobFiltersParams, PipelineInfoQueryParams, UpdateJobRequest
};
use bld_runner::VersionedPipeline;
use details::CronJobsEditDetails;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;
use schedule::CronJobsEditSchedule;
use std::collections::HashMap;

type SaveCronJob = (
    Option<String>,
    Option<String>,
    String,
    HashMap<String, RwSignal<String>>,
    HashMap<String, RwSignal<String>>,
);

fn hash_map_rw_signals(
    pipeline_items: HashMap<String, String>,
    mut cron_items: Option<HashMap<String, String>>,
) -> HashMap<String, RwSignal<String>> {
    pipeline_items
        .into_iter()
        .map(|(k, v)| {
            let value = cron_items
                .as_mut()
                .and_then(|c| c.remove(&k))
                .unwrap_or_else(|| v);

            (k, create_rw_signal(value))
        })
        .collect()
}

fn hash_map_strings(items: HashMap<String, RwSignal<String>>) -> HashMap<String, String> {
    items
        .into_iter()
        .map(|(k, v)| (k, v.get_untracked()))
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

async fn get_pipeline(name: Option<String>) -> Result<Option<VersionedPipeline>> {
    let name = name.ok_or_else(|| anyhow!("No name provided"))?;
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

async fn fetch_by_id(
    id: Option<String>,
    pipeline: WriteSignal<Option<VersionedPipeline>>,
    cron: WriteSignal<Option<CronJobResponse>>,
) {
    let cron_resp = get_cron(id)
        .await
        .map_err(|e| logging::console_error(e.to_string().as_str()))
        .ok();

    let pipeline_resp = get_pipeline(cron_resp.as_ref().map(|x| x.pipeline.clone()))
        .await
        .map_err(|e| logging::console_error(e.to_string().as_str()))
        .ok();

    if let Some(pipeline_resp) = pipeline_resp {
        pipeline.set(pipeline_resp);
    }

    cron.set(cron_resp);
}

async fn fetch_by_name(
    name: Option<String>,
    pipeline: WriteSignal<Option<VersionedPipeline>>,
    cron: WriteSignal<Option<CronJobResponse>>,
) {
    let job = name.as_ref().map(|name| CronJobResponse {
        pipeline: name.clone(),
        ..Default::default()
    });

    let pipeline_resp = get_pipeline(name)
        .await
        .map_err(|e| logging::console_error(e.to_string().as_str()))
        .ok();

    if let Some(pipeline_resp) = pipeline_resp {
        pipeline.set(pipeline_resp);
        cron.set(job);
    }
}

async fn update_cron(
    id: String,
    schedule: String,
    variables: Option<HashMap<String, String>>,
    environment: Option<HashMap<String, String>>,
) -> Result<()> {
    let data = UpdateJobRequest::new(id, schedule, variables, environment);

    let res = Client::builder()
        .build()?
        .patch("http://localhost:6080/v1/cron")
        .json(&data)
        .send()
        .await?;

    if res.status().is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let msg = format!("Request failed with status: {}", res.status());
        logging::console_error(&msg);
        bail!(msg)
    }
}

async fn insert_cron(
    name: String,
    schedule: String,
    variables: Option<HashMap<String, String>>,
    environment: Option<HashMap<String, String>>
) -> Result<()> {
    let data = AddJobRequest::new(schedule, name, variables, environment, false);

    let res = Client::builder()
        .build()?
        .post("http://localhost:6080/v1/cron")
        .json(&data)
        .send()
        .await?;

    if res.status().is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let msg = format!("Request failed with status: {}", res.status());
        logging::console_error(&msg);
        bail!(msg)
    }
}

#[component]
pub fn CronJobsEdit() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = move || params.with(|p| p.get("name").cloned());
    let is_new = move || params.with(|p| p.get("new").is_some());
    let (cron, set_cron) = create_signal(None);
    let (pipeline, set_pipeline) = create_signal(None);
    let schedule = create_rw_signal(String::new());
    let variables = create_rw_signal(HashMap::new());
    let environment = create_rw_signal(HashMap::new());

    create_resource(
        move || (id(), name(), is_new(), set_pipeline, set_cron),
        |(id, name, is_new, set_pipeline, set_cron)| async move {
            if is_new && name.is_some() {
                fetch_by_name(name, set_pipeline, set_cron).await;
            } else {
                fetch_by_id(id, set_pipeline, set_cron).await;
            }
        },
    );

    create_effect(move |_| {
        let (Some(cron), Some(pipeline)) = (cron.get(), pipeline.get()) else {
            return;
        };
        schedule.set(cron.schedule);
        let (vars, env) = pipeline.variables_and_environment();
        variables.set(hash_map_rw_signals(vars, cron.variables));
        environment.set(hash_map_rw_signals(env, cron.environment));
    });

    let save_action = create_action(|args: &SaveCronJob| {
        let (id, name, schedule, vars, env) = args;
        let id = id.as_ref().cloned();
        let name = name.as_ref().cloned();
        let schedule = schedule.clone();
        let vars = Some(hash_map_strings(vars.clone()));
        let env = Some(hash_map_strings(env.clone()));
        async move {
            if let Some(name) = name {
                let _ = insert_cron(name, schedule, vars, env).await;
            } else if let Some(id) = id {
                let _ = update_cron(id, schedule, vars, env).await;
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
                <CronJobsEditDetails
                    job=move || cron.get().unwrap()
                    save=move || {
                        save_action.dispatch((
                            id(),
                            name(),
                            schedule.get(),
                            variables.get(),
                            environment.get()))
                    }/>
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
