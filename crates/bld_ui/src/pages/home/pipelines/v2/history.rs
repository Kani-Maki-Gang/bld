use crate::{
    components::{button::Button, card::Card},
    pages::home::history::table::HistoryTable,
};
use bld_models::dtos::HistQueryParams;
use leptos::*;

#[component]
pub fn PipelineHistoryV2(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let params = move || {
        name.get().map(|n| HistQueryParams {
            name: Some(n),
            state: None,
            limit: 10000,
        })
    };

    let (refresh, set_refresh) = create_signal(());

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600px]">
                <div class="flex">
                    <div class="grow">
                        <div class="text-xl">
                            "History"
                        </div>
                        <div class="text-gray-400">
                            "The last 10k runs for the pipeline"
                        </div>
                    </div>
                    <div class="min-w-40">
                        <Button on:click={move |_| set_refresh.set(())}>"Refresh"</Button>
                    </div>
                </div>
                <HistoryTable params=params refresh=refresh />
            </div>
        </Card>
    }
}
