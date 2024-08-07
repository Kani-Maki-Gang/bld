use crate::{api, error::ErrorCard};
use anyhow::Result;
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::VersionedPipeline;
use leptos::*;
use leptos_router::*;

use super::v2::PipelineV2;

async fn get_pipeline(id: Option<String>) -> Result<VersionedPipeline> {
    let id = id.ok_or_else(|| anyhow::anyhow!("Id not provided as query parameter"))?;
    let params = PipelineInfoQueryParams::Id { id };
    api::print(params).await
}

#[component]
pub fn PipelineInfo() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = move || params.with(|p| p.get("name").cloned());

    let data = create_resource(
        move || id(),
        |id| async move { get_pipeline(id).await.map_err(|e| e.to_string()) },
    );

    view! {
        <Show
            when=move || matches!(data.get(), Some(Ok(VersionedPipeline::Version1(_))))
            fallback=|| view! {}
        >
            "TODO!"
        </Show>
        <Show
            when=move || matches!(data.get(), Some(Ok(VersionedPipeline::Version2(_))))
            fallback=|| view! {}
        >
            <PipelineV2 id=id name=name pipeline=move || data.get().unwrap().ok()/>
        </Show>
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || data.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || data.loading().get() fallback=|| view! {}>
            "Loading..."
        </Show>
    }
}
