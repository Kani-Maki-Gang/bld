use leptos::*;

#[derive(Debug, Clone)]
pub struct ButtonGroupItem {
    pub id: String,
    pub label: String,
}

#[component]
pub fn ButtonGroup(
    #[prop(into)] items: Signal<Vec<ButtonGroupItem>>,
    #[prop(into)] selected: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="inline-flex rounded-lg border border-gray-800 p-1 bg-slate-900">
            {move || items
                .get()
                .into_iter()
                .map(|x| if x.id == selected.get() {
                    view! {
                        <button
                            class="inline-block rounded-md px-4 py-2 text-sm text-gray-200 shadow-sm focus:relative bg-slate-800"
                            on:click=move |_e| selected.set(x.id.clone())>
                            {x.label}
                        </button>
                    }.into_view()
                } else {
                    view! {
                        <button
                            class="inline-block rounded-md px-4 py-2 text-sm text-gray-400 hover:text-gray-200 focus:relative"
                            on:click=move |_e| selected.set(x.id.clone())>
                            {x.label}
                        </button>
                    }.into_view()
                })
                .collect::<Vec<_>>()
            }
        </div>
    }
}
