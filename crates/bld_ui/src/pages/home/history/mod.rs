mod filters;
mod list;

use crate::components::card::Card;
use filters::HistoryFilters;
use list::HistoryList;
use leptos::*;

#[component]
pub fn History() -> impl IntoView {
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
                        <HistoryFilters />
                    </div>
                    <HistoryList />
                </div>
            </Card>
        </div>
    }
}
