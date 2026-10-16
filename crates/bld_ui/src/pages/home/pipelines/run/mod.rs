pub mod variables;

use crate::{
    api,
    components::{
        button::{ButtonSize, ButtonVariant, button_classes},
        card::{Card, Eyebrow},
        dag::WorkflowGraph,
        link::{BackLink, LinkButton},
        page::{LoadingState, PageHeader},
    },
    context::use_app_dialog,
    error::ErrorCard,
};
use anyhow::{Result, anyhow};
use bld_models::dtos::{ExecClientMessage, PipelineInfoQueryParams};
use bld_runner::{VersionedFile, files::v3::RunnerFile};
use leptos::*;
use leptos_router::*;
use std::collections::HashMap;
use variables::{InputField, RunPipelineVariables, field_values, file_fields};

async fn get_pipeline(id: Option<String>, name: Option<String>) -> Result<VersionedFile> {
    let params = match (id, name) {
        (Some(id), _) if !id.is_empty() => PipelineInfoQueryParams::Id { id },
        (_, Some(name)) => PipelineInfoQueryParams::Name { name },
        _ => return Err(anyhow!("No pipeline id or name provided in the url")),
    };
    let response = api::print(params).await?;
    Ok(serde_yaml_ng::from_str(&response)?)
}

#[component]
fn RunPlan(#[prop()] graph: WorkflowGraph) -> impl IntoView {
    let runs_on: HashMap<String, String> = graph
        .nodes
        .iter()
        .map(|n| (n.name.clone(), n.runs_on.clone()))
        .collect();
    let subtitle = format!(
        "{} jobs resolved into {} stages",
        graph.nodes.len(),
        graph.stages.len()
    );
    view! {
        <Card class="px-5 py-[18px]">
            <div class="text-[15px] font-semibold tracking-[-.01em]">"Run plan"</div>
            <div class="text-ink-muted text-[12.5px] mt-[3px]">{subtitle}</div>
            <div class="mt-3.5 flex flex-col">
                {graph
                    .stages
                    .iter()
                    .enumerate()
                    .map(|(i, stage)| {
                        let detail = if stage.len() > 1 {
                            format!("{} in parallel", stage.len())
                        } else {
                            stage.first().and_then(|n| runs_on.get(n)).cloned().unwrap_or_default()
                        };
                        view! {
                            <div class="flex items-center gap-[11px] py-[9px] border-t border-line-soft">
                                <span class="font-mono text-[11px] text-ink-faint w-3.5">
                                    {i + 1}
                                </span>
                                <span class="font-mono text-[12.5px] flex-1 min-w-0 break-words">
                                    {stage.join(", ")}
                                </span>
                                <span class="text-[11.5px] text-ink-muted shrink-0">{detail}</span>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </Card>
    }
}

#[component]
fn Capacity() -> impl IntoView {
    let data = create_resource(
        || (),
        |_| async move { api::running_pipelines().await.ok() },
    );
    let text = move || match data.get().flatten() {
        Some(kpi) if kpi.available_workers > 0 => format!(
            "{} of {} workers idle. This run starts as soon as you start it.",
            kpi.available_workers,
            kpi.count + kpi.available_workers
        ),
        Some(kpi) => format!(
            "All {} workers are busy. This run waits in the queue for a free worker.",
            kpi.count + kpi.available_workers
        ),
        None => "Worker data not available.".to_string(),
    };
    view! {
        <Card class="px-5 py-[18px]">
            <Eyebrow>"Capacity"</Eyebrow>
            <div class="mt-2.5 text-[13px] text-ink-soft">{text}</div>
        </Card>
    }
}

#[component]
fn RunForm(
    #[prop()] name: String,
    #[prop()] back: String,
    #[prop()] file: VersionedFile,
) -> impl IntoView {
    let dialog = use_app_dialog();
    let attempted = create_rw_signal(false);
    let graph = match &file {
        VersionedFile::Version3(RunnerFile::PipelineFileType(p)) if !p.jobs.is_empty() => {
            Some(WorkflowGraph::new(p))
        }
        _ => None,
    };
    let (inputs, env) = file_fields(file, None, None);
    let required: Vec<InputField> = inputs.iter().filter(|f| f.required).cloned().collect();
    let ready = move || required.iter().all(|f| !f.is_missing());
    let ready = Signal::derive(ready);

    let start_run = create_action(
        move |(name, inputs, env): &(String, HashMap<String, String>, HashMap<String, String>)| {
            let data = ExecClientMessage::EnqueueRun {
                name: name.clone(),
                inputs: Some(inputs.clone()),
                env: Some(env.clone()),
            };
            async move {
                match api::run(data).await {
                    Ok(id) => {
                        let nav = use_navigate();
                        nav(&format!("/monit?id={id}"), NavigateOptions::default());
                    }
                    Err(e) => dialog.error(e.to_string()),
                }
            }
        },
    );

    let has_inputs = !inputs.is_empty();
    let has_env = !env.is_empty();
    let inputs_for_start = inputs.clone();
    let env_for_start = env.clone();
    let run_name = name.clone();
    let back_link = back.clone();

    view! {
        <PageHeader
            title="Run pipeline"
            subtitle="Review the inputs resolved from bld expressions, then start the run."
            back=view! { <BackLink href=back_link>{name}</BackLink> }.into_view()
        >
            <LinkButton href=back variant=ButtonVariant::Ghost size=ButtonSize::Lg>
                "Cancel"
            </LinkButton>
            <button
                class=move || {
                    let base = button_classes(ButtonVariant::Primary, ButtonSize::Lg);
                    if ready.get() {
                        base
                    } else {
                        format!("{base} !bg-line-strong !text-ink-muted")
                    }
                }
                title=move || {
                    if ready.get() { "Start run" } else { "Required inputs are missing" }
                }
                on:click=move |_| {
                    if ready.get_untracked() {
                        attempted.set(false);
                        start_run
                            .dispatch((
                                run_name.clone(),
                                field_values(&inputs_for_start),
                                field_values(&env_for_start),
                            ));
                    } else {
                        attempted.set(true);
                    }
                }
            >
                "Start run"
            </button>
        </PageHeader>
        <div class="mt-5 grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-3.5 items-start">
            <div class="flex flex-col gap-3.5 min-w-0">
                <Card class="overflow-hidden">
                    {if has_inputs {
                        view! {
                            <RunPipelineVariables
                                title="Inputs"
                                subtitle="Values that differ from the pipeline default show in violet."
                                fields=inputs
                                attempted=attempted
                            />
                        }
                            .into_view()
                    } else {
                        view! {
                            <div class="px-5 py-4 text-[12.5px] text-ink-muted">
                                "This pipeline has no inputs."
                            </div>
                        }
                            .into_view()
                    }}
                </Card>
                {has_env
                    .then(|| {
                        view! {
                            <Card class="overflow-hidden">
                                <RunPipelineVariables
                                    title="Environment"
                                    subtitle="Environment variables given to every step."
                                    fields=env
                                />
                            </Card>
                        }
                    })}
            </div>
            <div class="flex flex-col gap-3.5">
                {graph.map(|graph| view! { <RunPlan graph=graph /> })} <Capacity />
            </div>
        </div>
    }
}

#[component]
pub fn RunPipeline() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = move || params.with(|p| p.get("name").cloned());

    let data = create_resource(
        move || (id(), name()),
        |(id, name)| async move { get_pipeline(id, name).await.map_err(|e| e.to_string()) },
    );

    move || match data.get() {
        None => view! { <LoadingState /> }.into_view(),
        Some(Err(e)) => view! { <ErrorCard error=e /> }.into_view(),
        Some(Ok(file)) => {
            let name = name().unwrap_or_default();
            let back = format!(
                "/files/info?id={}&name={}",
                id().unwrap_or_default(),
                name
            );
            view! { <RunForm name=name back=back file=file /> }.into_view()
        }
    }
}
