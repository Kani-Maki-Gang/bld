use leptos::*;
use crate::components::{card::Card, table::{Table, TableRow}};

#[component]
pub fn PipelineArtifactsV2(
    #[prop(into)] headers: Signal<Vec<View>>,
    #[prop(into)] rows: Signal<Vec<TableRow>>
) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600px]">
                <div class="text-xl">
                    "Artifacts"
                </div>
                <div class="text-gray-400">
                    "The configured artifact operations for this pipeline."
                </div>
                <Show
                    when=move || !rows.get().is_empty()
                    fallback=|| view! {
                        <div class="text-gray-400">
                            "No artifacts configured."
                        </div>
                    }>
                    <Table headers=headers rows=rows />
                </Show>
            </div>
        </Card>
    }
}
