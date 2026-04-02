mod filters;
pub mod table;

use crate::context::RefreshHistory;
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
        <div class="flex flex-col min-h-full">
            <div class="px-6 py-5 border-b border-zinc-800 flex items-center gap-4">
                <div class="grow">
                    <div class="text-lg font-semibold text-white">"History"</div>
                    <div class="text-xs text-zinc-500 mt-0.5">
                        "Pipeline runs ordered by start date"
                    </div>
                </div>
            </div>
            <div class="px-6 py-3 border-b border-zinc-800/60">
                <HistoryFilters state=state limit=limit pipeline=pipeline />
            </div>
            <div class="px-6 py-5">
                <HistoryTable params=params />
            </div>
        </div>
    }
}
