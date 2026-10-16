mod delete;
mod download;

use crate::{
    api,
    components::{
        card::Card,
        page::{EmptyState, LoadingState},
    },
    context::RefreshArtifacts,
    error::ErrorBanner,
};
use anyhow::Result;
use bld_models::dtos::{ArtifactResponse, ArtifactsQueryParams};
use leptos::*;

use {delete::ArtifactDeleteButton, download::ArtifactDownloadButton};

const GRID: &str = "grid grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)_150px_150px_70px] gap-3.5 px-5";

async fn get_artifacts(run_id: Option<String>) -> Result<Vec<ArtifactResponse>> {
    let run_id = run_id.ok_or_else(|| anyhow::anyhow!("Run id not provided"))?;
    api::artifacts(ArtifactsQueryParams { run_id }).await
}

#[component]
pub fn MonitArtifacts(#[prop(into)] run_id: Signal<Option<String>>) -> impl IntoView {
    let refresh = RefreshArtifacts(create_rw_signal(()));
    provide_context(refresh);

    let data = create_resource(
        move || (run_id.get(), refresh.0.get()),
        |(run_id, _)| async move { get_artifacts(run_id).await.map_err(|e| e.to_string()) },
    );

    view! {
        <Card class="mt-3.5 overflow-hidden">
            {move || match data.get() {
                None => view! { <LoadingState /> }.into_view(),
                Some(Err(e)) => {
                    view! {
                        <div class="p-5">
                            <ErrorBanner error=e />
                        </div>
                    }
                        .into_view()
                }
                Some(Ok(rows)) if rows.is_empty() => {
                    view! {
                        <EmptyState
                            title="No artifacts"
                            body="Artifacts that the steps of this run upload show here until they expire."
                        />
                    }
                        .into_view()
                }
                Some(Ok(rows)) => {
                    view! {
                        <div class="overflow-x-auto">
                            <div class="min-w-[720px]">
                                <div class=format!(
                                    "{GRID} py-[11px] border-b border-line font-mono text-[10.5px] tracking-[.09em] uppercase text-ink-faint",
                                )>
                                    <div>"Name"</div>
                                    <div>"Id"</div>
                                    <div>"Created"</div>
                                    <div>"Expires"</div>
                                    <div class="text-right">"Actions"</div>
                                </div>
                                {rows
                                    .into_iter()
                                    .map(|a| {
                                        view! {
                                            <div class=format!(
                                                "{GRID} py-3 border-b border-line-faint items-center",
                                            )>
                                                <div class="font-mono text-[12.5px] truncate">
                                                    {a.name.clone()}
                                                </div>
                                                <div class="font-mono text-[11px] text-ink-faint truncate">
                                                    {a.id.clone()}
                                                </div>
                                                <div class="font-mono text-xs text-ink-muted">
                                                    {a.date_created.clone()}
                                                </div>
                                                <div class="font-mono text-xs text-ink-muted">
                                                    {a.date_expires.clone()}
                                                </div>
                                                <div class="flex gap-1.5 justify-end">
                                                    <ArtifactDownloadButton
                                                        id=a.id.clone()
                                                        name=a.name.clone()
                                                    />
                                                    <ArtifactDeleteButton id=a.id name=a.name />
                                                </div>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                        .into_view()
                }
            }}
        </Card>
    }
}
