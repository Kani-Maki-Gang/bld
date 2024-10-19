mod cron;
mod details;
mod hist;
mod menu;
mod raw_file;

use crate::{
    api,
    components::card::Card,
    context::{RefreshCronJobs, RefreshHistory},
    error::ErrorCard,
};
use anyhow::Result;
use bld_models::dtos::PipelineInfoQueryParams;
use leptos::*;
use leptos_router::use_query_map;

use {cron::PipelineCron, details::PipelineDetails, hist::PipelineHist, raw_file::PipelineRawFile};

async fn get_pipeline(id: Option<String>) -> Result<String> {
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
    let selected_menu_item = create_rw_signal(menu::MenuItem::RawFile);

    provide_context(RefreshHistory(create_rw_signal(())));
    provide_context(RefreshCronJobs(create_rw_signal(())));

    view! {
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
            <Card class="h-full flex flex-col px-8 py-12">
                <PipelineDetails id=id name=name selected=selected_menu_item />
                <div class="grow">
                    <Show
                        when=move || matches!(selected_menu_item.get(), menu::MenuItem::RawFile)
                        fallback=|| view! {}
                    >
                        <PipelineRawFile raw_file=move || data.get().unwrap().unwrap() />
                    </Show>
                    <Show
                        when=move || matches!(selected_menu_item.get(), menu::MenuItem::History)
                        fallback=|| view! {}
                    >
                        <PipelineHist name=move || name() />
                    </Show>
                    <Show
                        when=move || matches!(selected_menu_item.get(), menu::MenuItem::Cron)
                        fallback=|| view! {}
                    >
                        <PipelineCron name=move || name() />
                    </Show>
                </div>
            </Card>
        </Show>
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || data.get().unwrap().unwrap_err() />
        </Show>
        <Show when=move || data.loading().get() fallback=|| view! {}>
            "Loading..."
        </Show>
    }
}
