mod cron;
mod hist;
mod raw_file;
mod workflow;

use super::actions::{PipelineCopyButton, PipelineDeleteButton, PipelineMoveButton};
use crate::{
    api,
    components::{
        badge::StateBadge,
        button::ButtonVariant,
        dag::WorkflowGraph,
        link::{BackLink, LinkButton},
        page::{LoadingState, PageHeader},
        tabs::{Tab, Tabs},
    },
    error::ErrorCard,
    pages::home::shared::health_state,
};
use anyhow::{Result, anyhow};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::{VersionedFile, files::v3::RunnerFile};
use leptos::*;
use leptos_router::use_query_map;

use {
    cron::PipelineCron, hist::PipelineHist, raw_file::PipelineRawFile, workflow::PipelineWorkflow,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum InfoTab {
    RawFile,
    Workflow,
    History,
    Cron,
}

#[derive(Clone)]
pub struct PipelineFile {
    pub raw: String,
    pub file: Option<VersionedFile>,
}

impl PipelineFile {
    pub fn summary(&self) -> String {
        match &self.file {
            Some(VersionedFile::Version1(_)) => "version 1 · sequential".to_string(),
            Some(VersionedFile::Version2(_)) => "version 2 · sequential".to_string(),
            Some(VersionedFile::Version3(RunnerFile::PipelineFileType(p))) => format!(
                "version 3 · type pipeline · {} inputs · {} jobs",
                p.inputs.len(),
                p.jobs.len()
            ),
            Some(VersionedFile::Version3(RunnerFile::ActionFileType(_))) => {
                "version 3 · type action".to_string()
            }
            None => "unrecognised file".to_string(),
        }
    }

    pub fn graph(&self) -> Option<WorkflowGraph> {
        match &self.file {
            Some(VersionedFile::Version3(RunnerFile::PipelineFileType(p)))
                if !p.jobs.is_empty() =>
            {
                Some(WorkflowGraph::new(p))
            }
            _ => None,
        }
    }

    fn version_chip(&self) -> Option<(&'static str, &'static str)> {
        match &self.file {
            Some(VersionedFile::Version3(RunnerFile::ActionFileType(_))) => {
                Some(("v3 · action", "bg-accent/[.12] text-accent-ink"))
            }
            Some(VersionedFile::Version3(_)) => {
                Some(("v3 · workflow", "bg-accent/[.12] text-accent-ink"))
            }
            Some(VersionedFile::Version2(_)) => {
                Some(("v2 · sequential", "bg-ink-muted/[.12] text-ink-muted"))
            }
            Some(VersionedFile::Version1(_)) => {
                Some(("v1 · sequential", "bg-ink-muted/[.12] text-ink-muted"))
            }
            None => None,
        }
    }
}

async fn get_pipeline(id: Option<String>, name: Option<String>) -> Result<PipelineFile> {
    let params = match (id, name) {
        (Some(id), _) if !id.is_empty() => PipelineInfoQueryParams::Id { id },
        (_, Some(name)) => PipelineInfoQueryParams::Name { name },
        _ => return Err(anyhow!("No pipeline id or name provided in the url")),
    };
    let raw = api::print(params).await?;
    let file = serde_yaml_ng::from_str(&raw).ok();
    Ok(PipelineFile { raw, file })
}

#[component]
pub fn PipelineInfo() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = Signal::derive(move || params.with(|p| p.get("name").cloned()).unwrap_or_default());
    let tab = create_rw_signal(InfoTab::RawFile);

    let data = create_local_resource(
        move || (id(), name.get()),
        |(id, name)| async move {
            get_pipeline(id, Some(name))
                .await
                .map_err(|e| e.to_string())
        },
    );

    let health = create_resource(
        move || name.get(),
        |name| async move {
            api::pipeline_health()
                .await
                .ok()
                .and_then(|h| h.into_iter().find(|x| x.pipeline == name))
        },
    );

    let id_label = Signal::derive(move || {
        health
            .get()
            .flatten()
            .map(|h| h.id)
            .or_else(id)
            .unwrap_or_default()
    });

    view! {
        <PageHeader
            title=name
            mono=true
            back=view! { <BackLink href="/files">"Files"</BackLink> }.into_view()
        >
            <LinkButton
                href=Signal::derive(move || {
                    format!("/files/run?id={}&name={}", id_label.get(), name.get())
                })
                variant=ButtonVariant::Primary
            >
                "Run"
            </LinkButton>
            <PipelineMoveButton id=id_label name=name redirect=true />
            <PipelineCopyButton name=name redirect=true />
            <PipelineDeleteButton name=name redirect=true />
        </PageHeader>
        <div class="flex items-center gap-3 mt-[7px] flex-wrap">
            <span class="font-mono text-[11.5px] text-ink-faint">{id_label}</span>
            {move || {
                health.get().flatten().map(|h| view! { <StateBadge state=health_state(&h) /> })
            }}
            {move || {
                data.get()
                    .and_then(|d| d.ok())
                    .and_then(|d| d.version_chip())
                    .map(|(label, class)| {
                        view! {
                            <span class=format!(
                                "inline-flex items-center h-[21px] px-2 rounded-md font-mono text-[11px] font-medium {class}",
                            )>{label}</span>
                        }
                    })
            }}
        </div>
        {move || match data.get() {
            None => view! { <LoadingState /> }.into_view(),
            Some(Err(e)) => {
                view! {
                    <div class="mt-5">
                        <ErrorCard error=e />
                    </div>
                }
                    .into_view()
            }
            Some(Ok(file)) => {
                let graph = file.graph();
                let has_graph = graph.is_some();
                let file = store_value(file);
                view! {
                    <Tabs>
                        <Tab
                            is_selected=Signal::derive(move || tab.get() == InfoTab::RawFile)
                            on:click=move |_| tab.set(InfoTab::RawFile)
                        >
                            "Raw file"
                        </Tab>
                        <Show when=move || has_graph fallback=|| view! {}>
                            <Tab
                                is_selected=Signal::derive(move || tab.get() == InfoTab::Workflow)
                                on:click=move |_| tab.set(InfoTab::Workflow)
                            >
                                "Workflow"
                            </Tab>
                        </Show>
                        <Tab
                            is_selected=Signal::derive(move || tab.get() == InfoTab::History)
                            on:click=move |_| tab.set(InfoTab::History)
                        >
                            "History"
                        </Tab>
                        <Tab
                            is_selected=Signal::derive(move || tab.get() == InfoTab::Cron)
                            on:click=move |_| tab.set(InfoTab::Cron)
                        >
                            "Cron jobs"
                        </Tab>
                    </Tabs>
                    {
                        let graph = graph.clone();
                        move || match tab.get() {
                            InfoTab::RawFile => {
                                view! { <PipelineRawFile file=file.get_value() /> }.into_view()
                            }
                            InfoTab::Workflow => {
                                match graph.clone() {
                                    Some(graph) => {
                                        view! { <PipelineWorkflow graph=graph /> }.into_view()
                                    }
                                    None => {
                                        view! { <PipelineRawFile file=file.get_value() /> }
                                            .into_view()
                                    }
                                }
                            }
                            InfoTab::History => view! { <PipelineHist name=name /> }.into_view(),
                            InfoTab::Cron => view! { <PipelineCron name=name /> }.into_view(),
                        }
                    }
                }
                    .into_view()
            }
        }}
    }
}
