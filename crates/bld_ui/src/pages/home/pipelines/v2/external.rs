use leptos::*;
use crate::components::{card::Card, list::{List, ListItem}};

#[component]
pub fn PipelineExternalV2(#[prop(into)] items: Signal<Vec<ListItem>>) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600]px">
                <div class="text-xl">
                    "External"
                </div>
                <div class="text-gray-400">
                    "The external pipelines local or server used by the pipeline."
                </div>
                <Show
                    when=move || !items.get().is_empty()
                    fallback= || view! {
                        <div class="text-gray-400">
                            "No external pipelines configured."
                        </div>
                    }>
                    <List items=items />
                </Show>
            </div>
        </Card>
    }
}
