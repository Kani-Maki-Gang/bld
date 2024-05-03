mod variables;

use std::collections::HashMap;

use crate::components::{badge::Badge, button::Button, card::Card};
use anyhow::{anyhow, bail, Result};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::VersionedPipeline;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;
use serde::Serialize;

use self::variables::{RunPipelineVariables, PipelineVariable};

#[derive(Serialize)]
enum RunParams {
    EnqueueRun {
        name: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    },
}

async fn get_pipeline(id: String) -> Result<Option<VersionedPipeline>> {
    let params = PipelineInfoQueryParams::Id { id };

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

async fn start_run(name: &str) {
    let name = name.to_string();
    let fut = async move {
        let data = RunParams::EnqueueRun {
            name,
            variables: None,
            environment: None,
        };

        let res = Client::builder()
            .build()?
            .post("http://localhost:6080/v1/run")
            .json(&data)
            .send()
            .await?;

        res.json::<String>().await.map_err(|e| anyhow!(e))
    };

    match fut.await {
        Ok(id) => {
            let nav = use_navigate();
            nav(&format!("/monit?id={}", id), NavigateOptions::default());
        }
        Err(e) => logging::console_error(&e.to_string()),
    }
}

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

#[component]
pub fn RunPipeline() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = move || params.with(|p| p.get("name").cloned());
    let (data, set_data) = create_signal(None);

    let _ = create_resource(
        move || (id(), set_data),
        |(id, set_data)| async move {
            let Some(id) = id else {
                return;
            };

            let value = get_pipeline(id)
                .await
                .map_err(|e| logging::console_error(&e.to_string()))
                .unwrap_or_else(|_| None);

            if value.is_some() {
                set_data.set(value);
            }
        },
    );

    let start_run = create_action(|name: &String| {
        let name = name.to_owned();
        async move { start_run(&name).await }
    });

    let variables = move || match data.get() {
        Some(VersionedPipeline::Version1(pipeline)) => into_pipeline_variables(pipeline.variables),
        Some(VersionedPipeline::Version2(pipeline)) => into_pipeline_variables(pipeline.variables),
        _ => Vec::with_capacity(0),
    };

    let environment = move || match data.get() {
        Some(VersionedPipeline::Version1(pipeline)) => {
            into_pipeline_variables(pipeline.environment)
        }
        Some(VersionedPipeline::Version2(pipeline)) => {
            into_pipeline_variables(pipeline.environment)
        }
        _ => Vec::with_capacity(0),
    };

    view! {
        <div class="flex flex-col gap-4">
            <Card>
                <div class="flex flex-col px-8 py-12 gap-y-4">
                    <div class="flex">
                        <div class="grow flex flex-col">
                            <div class="text-2xl">
                                "Start a new run"
                            </div>
                            <div class="text-gray-400">
                                {name}
                            </div>
                            <div class="flex gap-4">
                                <Show
                                    when=move || variables().is_empty()
                                    fallback=move || view!{}>
                                    <div class="flex-shrink">
                                        <Badge>
                                            "Pipeline has no variables"
                                        </Badge>
                                    </div>
                                </Show>
                                <Show
                                    when=move || environment().is_empty()
                                    fallback=move || view!{}>
                                    <div class="flex-shrink">
                                        <Badge>
                                            "Pipeline has no environment variables"
                                        </Badge>
                                    </div>
                                </Show>
                            </div>
                        </div>
                        <div class="min-w-40">
                            <Button on:click=move |_| start_run.dispatch(name().unwrap())>Start</Button>
                        </div>
                    </div>
                </div>
            </Card>
            <Show
                when=move || !variables().is_empty()
                fallback=move || view!{}>
                <RunPipelineVariables
                    title="Variables"
                    subtitle="The inputs that are provided based on bld expressions for each step."
                    items=variables />
            </Show>
            <Show
                when=move || !variables().is_empty()
                fallback=move || view!{}>
                <RunPipelineVariables
                    title="Environment"
                    subtitle="The environment variables that are provided based on bld expressions or bash expressions for each step."
                    items=environment />
            </Show>
        </div>
    }
}
