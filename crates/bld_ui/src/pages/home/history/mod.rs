mod filters;
pub mod table;

use crate::components::card::Card;
use bld_models::dtos::HistQueryParams;
use filters::HistoryFilters;
use leptos::*;
use table::HistoryTable;

fn get_params(
    state: Option<String>,
    limit: Option<String>,
    pipeline: Option<String>,
) -> Option<HistQueryParams> {
    let params = HistQueryParams {
        name: pipeline,
        state: state.filter(|x| x != "all"),
        limit: limit
            .as_ref()
            .map(|l| l.parse::<u64>().unwrap_or(100))
            .unwrap_or(100),
    };
    Some(params)
}

#[component]
pub fn History() -> impl IntoView {
    let state: RwSignal<Option<String>> = create_rw_signal(None);
    let limit = create_rw_signal(Some("100".to_string()));
    let pipeline: RwSignal<Option<String>> = create_rw_signal(None);
    let refresh = create_rw_signal(());

    let params = move || get_params(state.get(), limit.get(), pipeline.get());

    view! {
        <div class="flex flex-col gap-8 h-full">
            <Card>
                <div class="flex flex-col px-8 py-12">
                    <div class="flex justify-items-center gap-x-4 items-center">
                        <div class="grow flex flex-col">
                            <div class="text-2xl">
                                "History"
                            </div>
                            <div class="text-gray-400 mb-8">
                                "A list of pipelines and their state order by their start date"
                            </div>
                        </div>
                        <HistoryFilters
                            state=state
                            limit=limit
                            pipeline=pipeline
                            refresh=refresh />
                    </div>
                    <HistoryTable
                        params=params
                        refresh=refresh />
                </div>
            </Card>
        </div>
    }
}
