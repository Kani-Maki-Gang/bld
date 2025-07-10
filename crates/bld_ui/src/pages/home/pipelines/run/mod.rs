pub mod variables;

use std::collections::HashMap;

use crate::{
    api::{self, RunParams},
    components::{badge::Badge, button::Button, card::Card},
    context::{AppDialog, AppDialogContent},
    error::{ErrorCard, ErrorDialog},
};
use anyhow::{Result, anyhow};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::{
    VersionedFile,
    pipeline::{v1, v2},
    traits::IntoVariables,
};
use leptos::{html::Dialog, *};
use leptos_router::*;
use serde_yaml_ng::from_str;

use self::variables::RunPipelineVariables;

type RequestInterRepr = (
    String,
    HashMap<String, RwSignal<String>>,
    HashMap<String, RwSignal<String>>,
    NodeRef<Dialog>,
    RwSignal<Option<View>>,
);

async fn get_pipeline(id: Option<String>) -> Result<VersionedFile> {
    let id = id.ok_or_else(|| anyhow!("Pipeline id not provided in query"))?;
    let params = PipelineInfoQueryParams::Id { id };
    let response = api::print(params).await?;
    let pipeline = from_str(&response)?;
    Ok(pipeline)
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
    api::run(data).await
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
                    dialog_content.set(Some(
                        view! { <ErrorDialog error=move || e.to_string() dialog=dialog/> },
                    ));
                    let _ = dialog.get().map(|x| x.show_modal());
                }
            }
        }
    });

    create_effect(move |_| match data.get() {
        Some(Ok(VersionedFile::Version1(v1::Pipeline {
            variables: var,
            environment: env,
            ..
        })))
        | Some(Ok(VersionedFile::Version2(v2::Pipeline {
            variables: var,
            environment: env,
            ..
        }))) => {
            variables.set(hash_map_rw_signals(var));
            environment.set(hash_map_rw_signals(env));
        }

        Some(Ok(VersionedFile::Version3(file))) => {
            let (vars, env) = file.into_variables();
            if let Some(vars) = vars {
                variables.set(hash_map_rw_signals(vars));
            }

            if let Some(env) = env {
                environment.set(hash_map_rw_signals(env));
            }
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
                                    <Badge>"Pipeline has no variables"</Badge>
                                </Show>
                                <Show
                                    when=move || environment.get().is_empty()
                                    fallback=move || view! {}
                                >
                                    <Badge>"Pipeline has no environment variables"</Badge>
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
