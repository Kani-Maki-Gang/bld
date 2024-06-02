use leptos::*;

#[component]
pub fn Input(
    #[prop(into, optional)] id: String,
    #[prop(into, optional)] input_type: String,
    #[prop(into, optional)] min: i32,
    #[prop(into, optional)] max: i32,
    #[prop(into, optional)] placeholder: Option<String>,
    #[prop(into, optional)] disabled: Option<bool>,
    #[prop()] value: RwSignal<String>,
) -> impl IntoView {
    view! {
        <input
            id=id
            type=input_type
            min=min
            max=max
            class="h-[40px] w-full rounded-lg text-sm bg-slate-600 text-white px-4 py-2 focus:ring focus:ring-slate-500 focus:outline-none"
            placeholder=placeholder
            prop:disabled=disabled
            prop:value=move || value.get()
            on:input=move |ev| value.set(event_target_value(&ev))/>
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
            class="px-4 py-2 h-[40px] w-full rounded-lg bg-slate-600 text-sm focus:ring focus:ring-slate-500 focus:outline-none"
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
