use crate::{
    components::{button::IconButton, card::Card},
    context::RefreshHistory,
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
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12 gap-4">
                <div class="flex gap-4 items-start">
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
                <HistoryTable params=params/>
            </div>
        </Card>
    }
}
