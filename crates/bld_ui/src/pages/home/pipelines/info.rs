use anyhow::{bail, Result};
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::VersionedPipeline;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;
use crate::components::card::Card;

use super::v2::info::PipelineInfoV2;

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

#[component]
pub fn PipelineInfo() -> impl IntoView {
    let params = use_query_map();

    let id = move || {
        params.with(|p| p.get("id").cloned())
    };

    let name = move || {
        params.with(|p| p.get("name").cloned())
    };

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
        }
    );

    view! {
        <div class="flex flex-col spacing-4">
            <Show
                when=move || matches!(data.get(), Some(VersionedPipeline::Version1(_)))
                fallback=|| view! { }>
                "TODO!"
            </Show>
            <Show
                when=move || matches!(data.get(), Some(VersionedPipeline::Version2(_)))
                fallback=|| view! { }>
                <PipelineInfoV2 name=name pipeline=move || data.get() />
            </Show>
            <Show
                when=move || data.get().is_none()
                fallback=|| view! { }>
                "No pipeline found!"
            </Show>
        </div>
    }
}
