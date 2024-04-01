use leptos::*;

#[component]
pub fn Input(
    #[prop(optional)] input_type: Option<String>,
    #[prop(optional)] min: Option<i32>,
    #[prop(optional)] max: Option<i32>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop()] value: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <input
            type=input_type
            min=min
            max=max
            class="h-[40px] w-full rounded-lg shadow-sm sm:text-sm border-slate-500 bg-slate-600 text-white px-4 py-2"
            placeholder=placeholder
            prop:value={move || value.get().unwrap_or("".to_string())}
            on:input=move |ev| value.set(Some(event_target_value(&ev)))/>
    }
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
}

#[component]
pub fn Select(
    #[prop()] items: ReadSignal<Vec<SelectItem>>,
    #[prop()] value: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <select
            class="px-4 py-2 h-[40px] w-full rounded-lg border-slate-400 bg-slate-600 sm:text-sm"
            prop:value={move || value.get().unwrap_or("".to_string())}
            on:change=move |ev| value.set(Some(event_target_value(&ev))) >
            <For
                each=move || items.get()
                key=|state| state.value.clone()
                let:child>
                <option value={child.value}>{child.label}</option>
            </For>
        </select>
    }
}
