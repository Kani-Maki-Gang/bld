use crate::components::{
    badge::Badge,
    card::Card,
    table::{DataTable, Headers, Header, Body, Row, Cell},
};
use bld_runner::artifacts::v2;
use leptos::*;

#[component]
pub fn PipelineArtifactsV2(#[prop(into)] artifacts: Signal<Vec<v2::Artifacts>>) -> impl IntoView {
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
                    when=move || !artifacts.get().is_empty()
                    fallback=|| view! {
                        <div class="grid justify-items-center">
                            <Badge>"No artifacts configured."</Badge>
                        </div>
                    }>
                    <DataTable>
                        <Headers>
                            <Header>"Method"</Header>
                            <Header>"From"</Header>
                            <Header>"To"</Header>
                            <Header>"Ignore errors"</Header>
                            <Header>"After"</Header>
                        </Headers>
                        <Body>
                            <For
                                each=move ||  artifacts.get().into_iter().enumerate()
                                key=move |(i, _)| *i
                                let:child>
                                <Row>
                                    <Cell>{child.1.method}</Cell>
                                    <Cell>{child.1.from}</Cell>
                                    <Cell>{child.1.to}</Cell>
                                    <Cell>{child.1.ignore_errors}</Cell>
                                    <Cell>{child.1.after}</Cell>
                                </Row>
                            </For>
                        </Body>
                    </DataTable>
                </Show>
            </div>
        </Card>
    }
}
