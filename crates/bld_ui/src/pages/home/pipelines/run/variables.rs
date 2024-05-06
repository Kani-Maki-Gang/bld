use crate::components::{card::Card, input::Input};
use leptos::*;

#[derive(Debug, Clone)]
pub struct PipelineVariable {
    pub id: String,
    pub name: String,
    pub value: RwSignal<String>,
}

#[component]
pub fn RunPipelineVariables(
    #[prop(into)] title: String,
    #[prop(into)] subtitle: String,
    #[prop(into)] items: Signal<Vec<PipelineVariable>>,
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
                        each=move || items.get()
                        key=|i| i.id.clone()
                        let:item>
                        <div>
                            {item.name}
                        </div>
                        <div class="col-span-2">
                            <Input value=item.value />
                        </div>
                    </For>
                </div>
            </div>
        </Card>
    }
}
