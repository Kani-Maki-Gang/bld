use crate::{
    components::{
        button::Button,
        card::Card,
        tabs::{Tab, Tabs},
    },
    context::{RefreshCronJobs, RefreshHistory},
    pages::home::cron::CronJobsTable,
    pages::home::history::table::HistoryTable,
};
use bld_models::dtos::{HistQueryParams, JobFiltersParams};
use leptos::*;
use leptos_router::*;

#[component]
pub fn PipelineHistAndCronV2(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let selected_tab = create_rw_signal("history".to_string());

    let hist_params = move || {
        name.get().map(|n| HistQueryParams {
            name: Some(n),
            state: None,
            limit: 10000,
        })
    };

    let cron_params = move || {
        name.get().map(|n| JobFiltersParams {
            pipeline: Some(n),
            ..Default::default()
        })
    };

    let hist_refresh = use_context::<RefreshHistory>();
    let cron_refresh = use_context::<RefreshCronJobs>();

    let refresh = move || {
        if selected_tab.get() == "history" {
            let Some(RefreshHistory(refresh)) = hist_refresh else {
                logging::error!("RefreshHistory context not found");
                return;
            };
            refresh.set(());
        } else {
            let Some(RefreshCronJobs(refresh)) = cron_refresh else {
                logging::error!("RefreshHistory context not found");
                return;
            };
            refresh.set(());
        }
    };

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4 h-[600px]">
                <div class="flex gap-4">
                    <div class="grow">
                        <div class="text-xl">
                            "History and cron jobs"
                        </div>
                        <div class="text-gray-400">
                            "The cron jobs and last runs for the pipeline (with a 10k limit)"
                        </div>
                    </div>
                    <div class="min-w-40">
                        <Button on:click={move |_| refresh()}>"Refresh"</Button>
                    </div>
                    <Show when=move || selected_tab.get() == "cron" fallback=|| view!{}>
                        <div class="min-w-40">
                            <Button on:click=move |_| {
                                let nav = use_navigate();
                                nav(&format!("cron/insert?name={}", name.get().unwrap_or_default()), NavigateOptions::default());
                            }>
                                "Add new"
                            </Button>
                        </div>
                    </Show>
                </div>
                <Tabs>
                    <Tab
                        is_selected=move || selected_tab.get() == "history"
                        on:click=move |_| selected_tab.set("history".to_string())>
                        "History"
                    </Tab>
                    <Tab
                        is_selected=move || selected_tab.get() == "cron"
                        on:click=move |_| selected_tab.set("cron".to_string())>
                        "Cron jobs"
                    </Tab>
                </Tabs>
                <Show
                    when=move || selected_tab.get() == "history"
                    fallback=|| view!{}>
                    <HistoryTable params=hist_params />
                </Show>
                <Show
                    when=move || selected_tab.get() == "cron"
                    fallback=|| view!{}>
                    <CronJobsTable params=cron_params />
                </Show>
            </div>
        </Card>
    }
}
