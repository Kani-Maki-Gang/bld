use crate::components::{
    badge::Badge,
    card::Card,
    table::{Body, Cell, Header, Headers, Row, Table},
    tabs::{TabItem, Tabs},
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
                <For
                    each=move || data.get().into_iter().enumerate()
                    key=move |(i, _)| *i
                    let:child>
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
    #[prop(into)] variables: Signal<HashMap<String, String>>,
    #[prop(into)] environment: Signal<HashMap<String, String>>,
) -> impl IntoView {
    let (tabs, _set_tabs) = create_signal(vec![
        TabItem {
            id: "variables".to_string(),
            label: "Variables".to_string(),
        },
        TabItem {
            id: "environment".to_string(),
            label: "Environment".to_string(),
        },
    ]);
    let selected_tab = create_rw_signal("variables".to_string());

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600px]">
                <div class="text-xl">
                    "Inputs"
                </div>
                <div class="text-gray-400">
                    "The configured variables and environment variables for this pipeline."
                </div>
                <Tabs items=tabs selected=selected_tab />
                <Show
                    when=move || selected_tab.get() == "variables" && !variables.get().is_empty()
                    fallback=|| view! {}>
                    <VariablesTable data=variables />
                </Show>
                <Show
                    when=move || selected_tab.get() == "variables" && variables.get().is_empty()
                    fallback=|| view! {}>
                    <div class="grid justify-items-center">
                        <Badge>"No variables configured."</Badge>
                    </div>
                </Show>
                <Show
                    when=move || selected_tab.get() == "environment" && !environment.get().is_empty()
                    fallback=|| view! {}>
                    <VariablesTable data=environment />
                </Show>
                <Show
                    when=move || selected_tab.get() == "environment" && environment.get().is_empty()
                    fallback=|| view! {}>
                    <div class="grid justify-items-center">
                        <Badge>"No environment variables configured."</Badge>
                    </div>
                </Show>
            </div>
        </Card>
    }
}
