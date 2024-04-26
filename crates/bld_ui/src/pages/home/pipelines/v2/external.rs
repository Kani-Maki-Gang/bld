use crate::components::{
    badge::Badge,
    card::Card,
    list::{List, ListItem},
};
use leptos::*;

#[component]
pub fn PipelineExternalV2(#[prop(into)] items: Signal<Vec<ListItem>>) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600]px">
                <div class="flex flex-col">
                    <div class="text-xl">
                        "External"
                    </div>
                    <div class="text-gray-400">
                        "The external pipelines local or server used by this pipeline."
                    </div>
                </div>
                <Show
                    when=move || !items.get().is_empty()
                    fallback= || view! {
                        <div class="grid justify-items-center">
                            <Badge>"No external pipelines configured."</Badge>
                        </div>
                    }>
                    <List items=items />
                </Show>
            </div>
        </Card>
    }
}
