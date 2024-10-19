use crate::{
    components::button::IconButton, context::RefreshHistory,
    pages::home::history::table::HistoryTable,
};
use bld_models::dtos::HistQueryParams;
use leptos::*;

#[component]
pub fn PipelineHistV2(#[prop(into)] name: Signal<Option<String>>) -> impl IntoView {
    let params = move || {
        name.get().map(|n| HistQueryParams {
            name: Some(n),
            state: None,
            limit: 10000,
        })
    };

    let refresh = use_context::<RefreshHistory>();

    view! {
        <div class="flex flex-col">
            <div class="flex gap-4 items-start border border-slate-600 rounded-t-lg p-4">
                <div class="grow">
                    <div class="text-xl">"History"</div>
                    <div class="text-gray-400">
                        "The last runs for the pipeline (with a 10k limit)"
                    </div>
                </div>
                <IconButton
                    class="justify-end"
                    icon="iconoir-refresh-double"
                    on:click=move |_| {
                        let Some(refresh) = refresh else {
                            logging::error!("RefreshHistory context not found");
                            return;
                        };
                        refresh.set()
                    }
                />

            </div>
            <HistoryTable params=params />
        </div>
    }
}
