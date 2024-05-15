use crate::{
    components::{button::Button, card::Card, tabs::{TabItem, Tabs}},
    pages::home::history::table::HistoryTable,
    pages::home::cron_jobs::CronJobsTable,
};
use bld_models::dtos::{HistQueryParams, JobFiltersParams};
use leptos::*;

#[component]
pub fn PipelineHistAndCronV2(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let (tabs, _) = create_signal(vec![
        TabItem { id: "history".to_string(), label: "History".to_string() },
        TabItem { id: "cron".to_string(), label: "Cron jobs".to_string() }
    ]);
    let selected_tab = create_rw_signal("history".to_string());
    let (refresh, set_refresh) = create_signal(());

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

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4 h-[600px]">
                <div class="flex">
                    <div class="grow">
                        <div class="text-xl">
                            "History and cron jobs"
                        </div>
                        <div class="text-gray-400">
                            "The cron jobs and last runs for the pipeline (with a 10k limit)"
                        </div>
                    </div>
                    <div class="min-w-40">
                        <Button on:click={move |_| set_refresh.set(())}>"Refresh"</Button>
                    </div>
                </div>
                <Tabs items=tabs selected=selected_tab />
                <Show
                    when=move || selected_tab.get() == "history"
                    fallback=|| view!{}>
                    <HistoryTable params=hist_params refresh=refresh />
                </Show>
                <Show
                    when=move || selected_tab.get() == "cron"
                    fallback=|| view!{}>
                    <CronJobsTable params=cron_params refresh=refresh />
                </Show>
            </div>
        </Card>
    }
}
