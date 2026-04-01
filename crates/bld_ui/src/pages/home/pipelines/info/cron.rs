use crate::{
    components::{button::IconButton, colors::Colors}, context::RefreshCronJobs, pages::home::cron::CronJobsTable,
};
use bld_models::dtos::JobFiltersParams;
use leptos::*;
use leptos_router::*;

#[component]
pub fn PipelineCron(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let params = move || {
        name.get().map(|n| JobFiltersParams {
            pipeline: Some(n),
            ..Default::default()
        })
    };

    let refresh = use_context::<RefreshCronJobs>();

    view! {
        <div class="flex flex-col">
            <div class="flex gap-4 items-start p-4">
                <div class="grow">
                    <div class="text-lg font-semibold text-white">"Cron jobs"</div>
                    <div class="text-xs text-zinc-500 mt-0.5">"The cron jobs for the pipeline (with a 10k limit)"</div>
                </div>
                <IconButton
                    class="justify-end"
                    icon="iconoir-plus"
                    ghost=true
                    color=Colors::Violet
                    on:click=move |_| {
                        let nav = use_navigate();
                        nav(
                            &format!("cron/insert?name={}", name.get().unwrap_or_default()),
                            NavigateOptions::default(),
                        );
                    }
                />

                <IconButton
                    class="justify-end"
                    icon="iconoir-refresh-double"
                    ghost=true
                    color=Colors::Violet
                    on:click=move |_| {
                        let Some(refresh) = refresh else {
                            logging::error!("RefreshCronJobs context not found");
                            return;
                        };
                        refresh.set()
                    }
                />

            </div>
            <CronJobsTable params=params />
        </div>
    }
}
