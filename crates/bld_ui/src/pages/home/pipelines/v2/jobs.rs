use crate::components::{
    badge::Badge,
    card::Card,
    list::{List, ListItem},
    tabs::{TabItem, Tabs},
};
use leptos::*;
use std::collections::HashMap;

#[component]
pub fn PipelineJobsV2(#[prop(into)] jobs: Signal<HashMap<String, Vec<ListItem>>>) -> impl IntoView {
    let selected_tab = create_rw_signal(String::default());
    let tabs = move || {
        jobs.get()
            .keys()
            .map(|k| TabItem {
                id: k.clone(),
                label: k.clone(),
            })
            .collect::<Vec<TabItem>>()
    };

    selected_tab.update(|x: &mut String| {
        *x = jobs
            .get()
            .keys()
            .next()
            .map(|x| x.clone())
            .unwrap_or_default()
    });

    let items = move || {
        jobs.get()
            .get(&selected_tab.get())
            .cloned()
            .unwrap_or_default()
    };

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[500]px">
                <div class="text-xl">
                    "Jobs"
                </div>
                <div class="text-gray-400">
                    "The parallel jobs for this pipeline."
                </div>
                <Show
                    when=move || !tabs().is_empty()
                    fallback= || view! {
                        <div class="grid justify-items-center">
                            <Badge>"No jobs configured."</Badge>
                        </div>
                    }>
                    <Tabs items=tabs selected=selected_tab />
                    <List items=items />
                </Show>
            </div>
        </Card>
    }
}
