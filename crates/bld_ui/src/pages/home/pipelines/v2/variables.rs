use crate::components::{
    badge::Badge,
    card::Card,
    table::{Body, Cell, Header, Headers, Row, Table},
};
use leptos::*;
use std::collections::HashMap;

#[component]
fn VariablesTable(#[prop(into)] data: Signal<HashMap<String, String>>) -> impl IntoView {
    view! {
        <Table>
            <Headers>
                <Header>"Name"</Header>
                <Header>"Default value"</Header>
            </Headers>
            <Body>
                <For each=move || data.get().into_iter().enumerate() key=move |(i, _)| *i let:child>
                    <Row>
                        <Cell>{child.1.0}</Cell>
                        <Cell>{child.1.1}</Cell>
                    </Row>
                </For>
            </Body>
        </Table>
    }
}

#[component]
pub fn PipelineVariablesV2(
    #[prop(into)] title: String,
    #[prop(into)] subtitle: String,
    #[prop(into)] no_data_text: Signal<String>,
    #[prop(into)] variables: Signal<HashMap<String, String>>,
) -> impl IntoView {
    view! {
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12 gap-y-4">
                <div class="flex flex-col">
                    <div class="text-xl">{title}</div>
                    <div class="text-gray-400">{subtitle}</div>
                </div>
                <Show
                    when=move || !variables.get().is_empty()
                    fallback=move || {
                        view! {
                            <div class="grid justify-items-center">
                                <Badge>{move || no_data_text.get()}</Badge>
                            </div>
                        }
                    }
                >
                    <VariablesTable data=variables/>
                </Show>
            </div>
        </Card>
    }
}
