use crate::components::{
    badge::Badge,
    card::Card,
    table::{Table, TableRow},
    tabs::{TabItem, Tabs},
};
use leptos::*;
use std::collections::HashMap;

fn into_table_rows(data: HashMap<String, String>) -> Vec<TableRow> {
    data.into_iter()
        .map(|v| TableRow {
            columns: vec![v.0.into_view(), v.1.into_view()],
        })
        .collect::<Vec<TableRow>>()
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

    let (headers, _set_headers) =
        create_signal(vec!["Name".into_view(), "Default value".into_view()]);

    let vars = move || into_table_rows(variables.get());
    let env = move || into_table_rows(environment.get());

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
                    when=move || selected_tab.get() == "variables" && !vars().is_empty()
                    fallback=|| view! {}>
                    <Table headers=headers rows=vars />
                </Show>
                <Show
                    when=move || selected_tab.get() == "variables" && vars().is_empty()
                    fallback=|| view! {}>
                    <div class="grid justify-items-center">
                        <Badge>"No variables configured."</Badge>
                    </div>
                </Show>
                <Show
                    when=move || selected_tab.get() == "environment" && !env().is_empty()
                    fallback=|| view! {}>
                    <Table headers=headers rows=env />
                </Show>
                <Show
                    when=move || selected_tab.get() == "environment" && env().is_empty()
                    fallback=|| view! {}>
                    <div class="grid justify-items-center">
                        <Badge>"No environment variables configured."</Badge>
                    </div>
                </Show>
            </div>
        </Card>
    }
}