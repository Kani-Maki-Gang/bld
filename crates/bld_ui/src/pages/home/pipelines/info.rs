use crate::{components::card::Card, error::Error};
use anyhow::{bail, Result};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::VersionedPipeline;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

use super::v2::PipelineV2;

async fn get_pipeline(id: Option<String>) -> Result<VersionedPipeline> {
    let id = id.ok_or_else(|| anyhow::anyhow!("Id not provided as query parameter"))?;
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
        bail!(error)
    }
}

#[component]
pub fn PipelineInfo() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let name = move || params.with(|p| p.get("name").cloned());

    let data = create_resource(
        move || id(),
        |id| async move {
            get_pipeline(id).await.map_err(|e| {
                logging::console_error(&e.to_string());
                e.to_string()
            })
        },
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
            <div class="flex flex-col items-center">
                <Card class="container flex flex-col px-8 py-12">
                    <Error error=move || data.get().unwrap().unwrap_err()/>
                </Card>
            </div>
        </Show>
        <Show when=move || data.loading().get() fallback=|| view! {}>
            "Loading..."
        </Show>
    }
}
