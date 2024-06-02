use crate::components::{
    badge::Badge,
    card::Card,
    list::List,
    tabs::{Tab, Tabs},
};
use bld_runner::step::v2::BuildStep;
use leptos::{leptos_dom::logging, *};
use std::collections::HashMap;

#[component]
pub fn PipelineJobsV2(
    #[prop(into)] jobs: Signal<HashMap<String, Vec<BuildStep>>>,
) -> impl IntoView {
    let selected_tab = create_rw_signal(String::new());

    let jobs = move || {
        let data = jobs
            .get()
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    v.into_iter()
                        .map(|x| {
                            serde_yaml::to_string(&x)
                                .map_err(|e| logging::console_error(&format!("{:?}", e)))
                                .unwrap_or_default()
                        })
                        .collect::<Vec<String>>(),
                )
            })
            .collect::<HashMap<String, Vec<String>>>();

        data
    };

    let items = move || {
        let key = selected_tab.get();
        jobs()
            .get(&key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(move |(i, x)| (format!("{key}_{i}"), x))
    };

    let _ = watch(
        move || jobs(),
        move |x, _, _| {
            selected_tab.set(x.keys().next().cloned().unwrap_or_default());
        },
        true,
    );

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600px]">
                <div class="flex flex-col">
                    <div class="text-xl">
                        "Jobs"
                    </div>
                    <div class="text-gray-400">
                        "The parallel jobs for this pipeline."
                    </div>
                </div>
                <Show
                    when=move || !jobs().is_empty()
                    fallback= || view! {
                        <div class="grid justify-items-center">
                            <Badge>"No jobs configured."</Badge>
                        </div>
                    }>
                    "Hello"
                    <Tabs>
                        <For
                            each=move || jobs()
                                .into_keys()
                                .enumerate()
                                .map(|(i, x)| (i, x.clone(), x.clone(), x))
                            key=|(i, _, _, _)| *i
                            let:child>
                            <Tab
                                is_selected=move || selected_tab.get() == *child.1
                                on:click=move |_| selected_tab.set(child.2.to_owned())>
                                {child.3}
                            </Tab>
                        </For>
                    </Tabs>
                    <List>
                        <For
                            each=move || items()
                            key=|(k, _)| k.clone()
                            let:child>
                            <pre class="text-sm text-gray-200 p-4 rounded-lg bg-slate-800">
                                {child.1}
                            </pre>
                        </For>
                    </List>
                </Show>
            </div>
        </Card>
    }
}
