mod filters;
pub mod table;

use crate::{components::card::Card, context::RefreshHistory};
use bld_models::dtos::HistQueryParams;
use filters::HistoryFilters;
use leptos::*;
use table::HistoryTable;

fn get_params(state: Option<String>, limit: String, pipeline: String) -> Option<HistQueryParams> {
    let params = HistQueryParams {
        name: if pipeline.is_empty() {
            None
        } else {
            Some(pipeline)
        },
        state: state.filter(|x| x != "all"),
        limit: limit.parse::<u64>().unwrap_or(100),
    };
    Some(params)
}

#[component]
pub fn History() -> impl IntoView {
    let state: RwSignal<Option<String>> = create_rw_signal(None);
    let limit = create_rw_signal("100".to_string());
    let pipeline: RwSignal<String> = create_rw_signal(String::new());
    let params = move || get_params(state.get(), limit.get(), pipeline.get());

    provide_context(RefreshHistory(create_rw_signal(())));

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
                            pipeline=pipeline />
                    </div>
                    <HistoryTable params=params />
                </div>
            </Card>
        </div>
    }
}
