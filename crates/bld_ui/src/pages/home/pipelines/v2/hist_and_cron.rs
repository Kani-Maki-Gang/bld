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

#[derive(Copy, Clone)]
pub enum TabType {
    History,
    Cron,
}

#[component]
pub fn PipelineHistAndCronV2(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let selected_tab = create_rw_signal(TabType::History);

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
        if matches!(selected_tab.get(), TabType::History) {
            let Some(hist) = hist_refresh else {
                logging::error!("RefreshHistory context not found");
                return;
            };
            hist.set();
        } else {
            let Some(cron) = cron_refresh else {
                logging::error!("RefreshHistory context not found");
                return;
            };
            cron.set();
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
                    <Show when=move || matches!(selected_tab.get(), TabType::History) fallback=|| view!{}>
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
                        is_selected=move || matches!(selected_tab.get(), TabType::History)
                        on:click=move |_| selected_tab.set(TabType::History)>
                        "History"
                    </Tab>
                    <Tab
                        is_selected=move || matches!(selected_tab.get(), TabType::Cron)
                        on:click=move |_| selected_tab.set(TabType::Cron)>
                        "Cron jobs"
                    </Tab>
                </Tabs>
                <Show
                    when=move || matches!(selected_tab.get(), TabType::History)
                    fallback=move || view!{
                        <CronJobsTable params=cron_params />
                    }>
                    <HistoryTable params=hist_params />
                </Show>
            </div>
        </Card>
    }
}
