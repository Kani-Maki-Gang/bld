use crate::components::{
    badge::Badge,
    card::Card,
    table::{Body, Cell, Header, Headers, Row, Table},
    tabs::{Tab, Tabs},
};
use leptos::*;
use std::collections::HashMap;

#[derive(Copy, Clone)]
enum TabType {
    Variables,
    Environment,
}

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
    let selected_tab = create_rw_signal(TabType::Variables);
    let view_is_variables = move || matches!(selected_tab.get(), TabType::Variables);
    let view_is_environment = move || matches!(selected_tab.get(), TabType::Environment);
    let set_view_to_variables = move || selected_tab.set(TabType::Variables);
    let set_view_to_environment = move || selected_tab.set(TabType::Environment);

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600px]">
                <div class="text-xl">
                    "Inputs"
                </div>
                <div class="text-gray-400">
                    "The configured variables and environment variables for this pipeline."
                </div>
                <Tabs>
                    <Tab
                        is_selected=move || view_is_variables()
                        on:click=move |_| set_view_to_variables()>
                        "Variables"
                    </Tab>
                    <Tab
                        is_selected=move || view_is_environment()
                        on:click=move |_| set_view_to_environment()>
                        "Environment"
                    </Tab>
                </Tabs>
                <Show
                    when=move || view_is_variables()  && !variables.get().is_empty()
                    fallback=|| view! {}>
                    <VariablesTable data=variables />
                </Show>
                <Show
                    when=move || view_is_variables() && variables.get().is_empty()
                    fallback=|| view! {}>
                    <div class="grid justify-items-center">
                        <Badge>"No variables configured."</Badge>
                    </div>
                </Show>
                <Show
                    when=move || view_is_environment() && !environment.get().is_empty()
                    fallback=|| view! {}>
                    <VariablesTable data=environment />
                </Show>
                <Show
                    when=move || view_is_environment() && environment.get().is_empty()
                    fallback=|| view! {}>
                    <div class="grid justify-items-center">
                        <Badge>"No environment variables configured."</Badge>
                    </div>
                </Show>
            </div>
        </Card>
    }
}
