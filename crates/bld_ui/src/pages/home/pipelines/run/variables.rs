use crate::components::{card::Card, input::Input};
use leptos::*;
use std::collections::HashMap;

#[component]
pub fn RunPipelineVariables(
    #[prop(into)] title: String,
    #[prop(into)] subtitle: String,
    #[prop(into)] items: Signal<HashMap<String, RwSignal<String>>>,
) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-6 py-5 gap-4">
                <div>
                    <div class="text-base font-semibold text-white">{title}</div>
                    <div class="text-xs text-zinc-500 mt-0.5">{subtitle}</div>
                </div>
                <div class="grid grid-cols-3 items-center gap-4">
                    <For each=move || items.get().into_iter().enumerate() key=|(i, _)| *i let:item>
                        <div class="text-sm text-zinc-400">{item.1.0}</div>
                        <div class="col-span-2">
                            <Input value=item.1.1/>
                        </div>
                    </For>
                </div>
            </div>
        </Card>
    }
}
