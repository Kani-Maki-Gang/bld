use leptos::*;

#[derive(Debug, Clone)]
pub struct TabItem {
    pub id: String,
    pub label: String,
}

#[component]
pub fn Tabs(
    #[prop(into)] items: Signal<Vec<TabItem>>,
    #[prop(into)] selected: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="hidden sm:block">
            <nav class="flex gap-6" aria-label="Tabs">
                {move || items
                    .get()
                    .into_iter()
                    .map(|x| if x.id == selected.get() {
                        view! {
                            <button
                                class="shrink-0 rounded-lg px-4 py-2 text-sm font-medium text-gray-200 bg-slate-800"
                                on:click=move |_e| selected.set(x.id.clone())>
                                {x.label}
                            </button>
                        }.into_view()
                    } else {
                        view! {
                            <button
                                class="shrink-0 rounded-lg px-4 py-2 text-sm font-medium text-gray-400 hover:text-gray-200"
                                on:click=move |_e| selected.set(x.id.clone())>
                                {x.label}
                            </button>
                        }.into_view()
                    })
                    .collect::<Vec<_>>()
                }
            </nav>
        </div>
    }
}
