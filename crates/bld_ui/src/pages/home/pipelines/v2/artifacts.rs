use crate::components::{
    badge::Badge,
    card::Card,
    table::{Table, TableRow},
};
use leptos::*;

#[component]
pub fn PipelineArtifactsV2(
    #[prop(into)] headers: Signal<Vec<View>>,
    #[prop(into)] rows: Signal<Vec<TableRow>>,
) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600px]">
                <div class="flex flex-col">
                    <div class="text-xl">
                        "Artifacts"
                    </div>
                    <div class="text-gray-400">
                        "The configured artifact operations for this pipeline."
                    </div>
                </div>
                <Show
                    when=move || !rows.get().is_empty()
                    fallback=|| view! {
                        <div class="grid justify-items-center">
                            <Badge>"No artifacts configured."</Badge>
                        </div>
                    }>
                    <Table headers=headers rows=rows />
                </Show>
            </div>
        </Card>
    }
}
