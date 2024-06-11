use crate::components::{
    card::Card,
    tabs::{Tab, Tabs, TabsDirection},
};
use leptos::*;

#[derive(Clone, Default, Eq, PartialEq)]
pub enum MenuItem {
    #[default]
    Jobs,
    External,
    Variables,
    Environment,
    Artifacts,
    History,
    Cron,
    RawFile,
}

fn get_menu_items() -> Vec<RwSignal<(MenuItem, String)>> {
    vec![
        create_rw_signal((MenuItem::Jobs, "Jobs".to_string())),
        create_rw_signal((MenuItem::External, "External".to_string())),
        create_rw_signal((MenuItem::Variables, "Variables".to_string())),
        create_rw_signal((MenuItem::Environment, "Environment".to_string())),
        create_rw_signal((MenuItem::Artifacts, "Artifacts".to_string())),
        create_rw_signal((MenuItem::History, "History".to_string())),
        create_rw_signal((MenuItem::Cron, "Cron jobs".to_string())),
        create_rw_signal((MenuItem::RawFile, "Raw file".to_string())),
    ]
}

#[component]
pub fn PipelinesV2Menu(#[prop(into)] selected: RwSignal<MenuItem>) -> impl IntoView {
    let items = Signal::derive(get_menu_items);
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <Tabs direction=move || TabsDirection::Vertical>
                    <For each=move || items.get().into_iter().enumerate() key=|(i, _)| *i let:child>
                        <Tab
                            is_selected=move || selected.get() == child.1.get().0
                            on:click=move |_| selected.set(child.1.get().0)
                        >
                            {child.1.get().1}
                        </Tab>
                    </For>
                </Tabs>
            </div>
        </Card>
    }
}
