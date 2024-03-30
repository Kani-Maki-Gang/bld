use leptos::*;

#[component]
pub fn Input(
    #[prop(optional)] input_type: Option<String>,
    #[prop(optional)] min: Option<i32>,
    #[prop(optional)] max: Option<i32>,
    #[prop(optional)] placeholder: Option<String>,
) -> impl IntoView {
    view! {
        <input
            type=input_type
            min=min
            max=max
            class="border border-slate-800 bg-slate-600 rounded p-2 min-h-[45px] w-full"
            placeholder=placeholder />
    }
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
}

#[component]
pub fn Select(#[prop()] items: ReadSignal<Vec<SelectItem>>) -> impl IntoView {
    view! {
        <select class="border border-slate-800 bg-slate-600 rounded min-h-[45px] px-2 py-3">
            <For
                each=move || items.get()
                key=|state| state.value.clone()
                let:child>
                <option value={child.value}>{child.label}</option>
            </For>
        </select>
    }
}
