pub mod variables;

use std::collections::HashMap;

use crate::{
    components::{badge::Badge, button::Button, card::Card},
    context::{AppDialog, AppDialogContent},
    error::{ErrorCard, ErrorDialog}
};
use anyhow::{anyhow, bail, Result};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::{
    pipeline::{v1, v2},
    VersionedPipeline,
};
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;
use serde::Serialize;

use self::variables::RunPipelineVariables;

type RequestInterRepr = (
    String,
    HashMap<String, RwSignal<String>>,
    HashMap<String, RwSignal<String>>,
    NodeRef<Dialog>,
    RwSignal<Option<View>>,
);

#[derive(Serialize)]
enum RunParams {
    EnqueueRun {
        name: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    },
}

async fn get_pipeline(id: Option<String>) -> Result<VersionedPipeline> {
    let id = id.ok_or_else(|| anyhow!("Pipeline id not provided in query"))?;
    let params = PipelineInfoQueryParams::Id { id };

    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/print")
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?;

    let status = res.status();
    if status.is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        let body = res.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_error(&error);
        bail!(error)
    }
}

async fn start_run(
    name: &str,
    vars: HashMap<String, String>,
    env: HashMap<String, String>,
) -> Result<String> {
    let data = RunParams::EnqueueRun {
        name: name.to_string(),
        variables: Some(vars),
        environment: Some(env),
    };

    let res = Client::builder()
        .build()?
        .post("http://localhost:6080/v1/run")
        .json(&data)
        .send()
        .await?;

    let status = res.status();
    if status.is_success() {
        Ok(res.json::<String>().await?)
    } else {
        let body = res.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_error(&error);
        bail!(error)
    }
}

fn hash_map_rw_signals(items: HashMap<String, String>) -> HashMap<String, RwSignal<String>> {
    items
        .into_iter()
        .map(|(k, v)| (k, create_rw_signal(v)))
        .collect()
}

fn hash_map_strings(items: HashMap<String, RwSignal<String>>) -> HashMap<String, String> {
    items
        .into_iter()
        .map(|(k, v)| (k, v.get_untracked()))
        .collect()
}

#[component]
pub fn RunPipeline() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = move || params.with(|p| p.get("name").cloned());
    let variables = create_rw_signal(HashMap::new());
    let environment = create_rw_signal(HashMap::new());
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();

    let data = create_resource(
        move || id(),
        |id| async move { get_pipeline(id).await.map_err(|e| e.to_string()) },
    );

    let start_run = create_action(|args: &RequestInterRepr| {
        let (name, vars, env, dialog, dialog_content) = args.clone();
        let vars = hash_map_strings(vars);
        let env = hash_map_strings(env);
        async move {
            let result = start_run(&name, vars, env).await;
            match result {
                Ok(id) => {
                    let nav = use_navigate();
                    nav(&format!("/monit?id={}", id), NavigateOptions::default());
                }
                Err(e) => {
                    dialog_content.set(Some(view! { <ErrorDialog error=move || e.to_string() dialog=dialog/> }));
                    let _ = dialog.get().map(|x| x.show_modal());
                }
            }
        }
    });

    create_effect(move |_| match data.get() {
        Some(Ok(VersionedPipeline::Version1(v1::Pipeline {
            variables: var,
            environment: env,
            ..
        })))
        | Some(Ok(VersionedPipeline::Version2(v2::Pipeline {
            variables: var,
            environment: env,
            ..
        }))) => {
            variables.set(hash_map_rw_signals(var));
            environment.set(hash_map_rw_signals(env));
        }
        _ => {}
    });

    view! {
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || data.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
            <div class="flex flex-col gap-4">
                <Card class="px-8 py-12">
                    <div class="flex">
                        <div class="grow flex flex-col">
                            <div class="text-2xl">"Start a new run"</div>
                            <div class="text-gray-400">{name}</div>
                            <div class="flex gap-4">
                                <Show
                                    when=move || variables.get().is_empty()
                                    fallback=move || view! {}
                                >
                                    <div class="flex-shrink">
                                        <Badge>"Pipeline has no variables"</Badge>
                                    </div>
                                </Show>
                                <Show
                                    when=move || environment.get().is_empty()
                                    fallback=move || view! {}
                                >
                                    <div class="flex-shrink">
                                        <Badge>"Pipeline has no environment variables"</Badge>
                                    </div>
                                </Show>
                            </div>
                        </div>
                        <div class="min-w-40">
                            <Button on:click=move |_| {
                                let Some(AppDialog(dialog)) = app_dialog else {
                                    return;
                                };
                                let Some(AppDialogContent(content)) = app_dialog_content else {
                                    return;
                                };
                                start_run
                                    .dispatch((
                                        name().unwrap(),
                                        variables.get(),
                                        environment.get(),
                                        dialog,
                                        content,
                                    ))
                            }>"Start"</Button>
                        </div>
                    </div>
                </Card>
                <Show when=move || !variables.get().is_empty() fallback=move || view! {}>
                    <RunPipelineVariables
                        title="Variables"
                        subtitle="The inputs that are provided based on bld expressions for each step."
                        items=variables
                    />
                </Show>
                <Show when=move || !variables.get().is_empty() fallback=move || view! {}>
                    <RunPipelineVariables
                        title="Environment"
                        subtitle="The environment variables that are provided based on bld expressions or bash expressions for each step."
                        items=environment
                    />
                </Show>
            </div>
        </Show>
    }
}
