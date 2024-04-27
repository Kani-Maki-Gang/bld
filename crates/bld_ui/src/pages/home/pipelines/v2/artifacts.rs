use crate::components::{
    badge::Badge,
    card::Card,
    table::{Table, TableRow},
};
use bld_runner::artifacts::v2;
use leptos::*;

#[component]
pub fn PipelineArtifactsV2(
    #[prop(into)] artifacts: Signal<Vec<v2::Artifacts>>
) -> impl IntoView {
    let headers = Signal::from(|| {
        vec![
            "Method".into_view(),
            "From".into_view(),
            "To".into_view(),
            "Ignore errors".into_view(),
            "After".into_view(),
        ]
    });

    let rows = move || {
        artifacts
            .get()
            .into_iter()
            .map(|x| TableRow {
                columns: vec![
                    x.method.into_view(),
                    x.from.into_view(),
                    x.to.into_view(),
                    x.ignore_errors.into_view(),
                    x.after.into_view(),
                ],
            })
            .collect::<Vec<TableRow>>()
    };

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
                    when=move || !rows().is_empty()
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
