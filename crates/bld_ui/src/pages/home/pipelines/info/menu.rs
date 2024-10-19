use crate::components::tabs::{Tab, Tabs, TabsDirection};
use leptos::*;

#[derive(Clone, Default, Eq, PartialEq)]
pub enum MenuItem {
    #[default]
    RawFile,
    History,
    Cron,
}

fn get_menu_items() -> Vec<RwSignal<(MenuItem, String)>> {
    vec![
        create_rw_signal((MenuItem::RawFile, "Raw file".to_string())),
        create_rw_signal((MenuItem::History, "History".to_string())),
        create_rw_signal((MenuItem::Cron, "Cron jobs".to_string())),
    ]
}

#[component]
pub fn PipelinesV2Menu(#[prop(into)] selected: RwSignal<MenuItem>) -> impl IntoView {
    let items = Signal::derive(get_menu_items);
    view! {
        <div class="flex flex-col">
            <Tabs direction=move || TabsDirection::Horizontal>
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
    }
}
