use crate::{
    components::{button::IconButton, card::Card},
    context::RefreshCronJobs,
    pages::home::cron::CronJobsTable,
};
use bld_models::dtos::JobFiltersParams;
use leptos::*;
use leptos_router::*;

#[component]
pub fn PipelineCronV2(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let params = move || {
        name.get().map(|n| JobFiltersParams {
            pipeline: Some(n),
            ..Default::default()
        })
    };

    let refresh = use_context::<RefreshCronJobs>();

    view! {
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12 gap-4 overflow-auto">
                <div class="flex gap-4">
                    <div class="grow">
                        <div class="text-xl">
                            "Cron jobs"
                        </div>
                        <div class="text-gray-400">
                            "The cron jobs for the pipeline (with a 10k limit)"
                        </div>
                    </div>
                    <IconButton class="justify-end" icon="iconoir-refresh-double" on:click=move |_| {
                        let Some(refresh) = refresh else {
                            logging::error!("RefreshCronJobs context not found");
                            return;
                        };
                        refresh.set()
                    } />
                    <IconButton class="justify-end" icon="iconoir-plus" on:click=move |_| {
                        let nav = use_navigate();
                        nav(&format!("cron/insert?name={}", name.get().unwrap_or_default()), NavigateOptions::default());
                    } />
                </div>
                <CronJobsTable params=params />
            </div>
        </Card>
    }
}
