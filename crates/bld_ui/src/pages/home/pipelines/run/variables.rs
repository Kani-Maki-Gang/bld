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
            <div class="flex flex-col px-8 py-12 gap-4 max-h-[600px]">
                <div class="text-2xl">
                    {title}
                </div>
                <div class="text-gray-400 mb-8">
                    {subtitle}
                </div>
                <div class="grid grid-cols-3 gap-4">
                    <For
                        each=move || items.get().into_iter().enumerate()
                        key=|(i, _)| *i
                        let:item>
                        <div>
                            {item.1.0}
                        </div>
                        <div class="col-span-2">
                            <Input value=item.1.1 />
                        </div>
                    </For>
                </div>
            </div>
        </Card>
    }
}
