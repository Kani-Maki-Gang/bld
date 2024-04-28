use std::collections::HashMap;

use crate::components::{badge::Badge, button::Button, card::Card};
use anyhow::{anyhow, bail, Result};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::VersionedPipeline;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;
use serde::Serialize;

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
        Some(VersionedPipeline::Version1(pipeline)) => pipeline.variables,
        Some(VersionedPipeline::Version2(pipeline)) => pipeline.variables,
        _ => HashMap::new(),
    };

    let environment = move || match data.get() {
        Some(VersionedPipeline::Version1(pipeline)) => pipeline.environment,
        Some(VersionedPipeline::Version2(pipeline)) => pipeline.environment,
        _ => HashMap::new(),
    };

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96">
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
                <Show
                    when=move || !variables().is_empty()
                    fallback=move || view!{}>
                    <div class="col-span-2">
                        <span class="text-xl">
                            "Variables"
                        </span>
                    </div>
                    <div class="grid grid-cols-3 gap-4 border border-lg rounded rounded-lg p-4">
                        <For
                            each=move || variables().into_iter().enumerate()
                            key=|(i, _)| *i
                            let:child>
                            <div class="text-gray-400">
                                {child.1.0}
                            </div>
                            <div class="col-span-2 text-gray-200">
                                {child.1.1}
                            </div>
                        </For>
                    </div>
                </Show>
                <Show
                    when=move || !environment().is_empty()
                    fallback=move || view!{}>
                    <div>
                        <span class="text-xl">
                            "Environment"
                        </span>
                    </div>
                    <div class="grid grid-cols-3 gap-4 border border-lg rounded rounded-lg p-4">
                        <For
                            each=move || environment().into_iter().enumerate()
                            key=|(i, _)| *i
                            let:child>
                            <div class="text-gray-400">
                                {child.1.0}
                            </div>
                            <div class="col-span-2 text-gray-200">
                                {child.1.1}
                            </div>
                        </For>
                    </div>
                </Show>
            </div>
        </Card>
    }
}
