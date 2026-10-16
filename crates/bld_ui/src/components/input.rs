use leptos::*;

const INPUT_CLASS: &str = "w-full h-9 rounded-[9px] border border-line-strong bg-raised px-[11px] text-[12.5px] text-ink placeholder:text-ink-faint outline-none transition-shadow focus:border-accent focus:shadow-[0_0_0_3px_rgba(108,92,245,.14)] disabled:bg-[#101218] disabled:text-ink-muted disabled:border-line";

#[component]
pub fn Input(
    #[prop(into, optional)] id: String,
    #[prop(into, optional)] input_type: String,
    #[prop(into, optional)] placeholder: Option<String>,
    #[prop(into, optional)] disabled: Option<bool>,
    #[prop(into, optional)] mono: bool,
    #[prop()] value: RwSignal<String>,
) -> impl IntoView {
    let class = if mono {
        format!("{INPUT_CLASS} font-mono")
    } else {
        INPUT_CLASS.to_string()
    };
    view! {
        <input
            id=id
            type=input_type
            class=class
            placeholder=placeholder
            prop:disabled=disabled
            prop:value=move || value.get()
            on:input=move |ev| value.set(event_target_value(&ev))
        />
    }
}

#[component]
pub fn SearchInput(
    #[prop(into)] placeholder: String,
    #[prop()] value: RwSignal<String>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let class = format!(
        "h-8 flex items-center gap-2 px-[11px] border border-line-strong rounded-[9px] bg-surface focus-within:border-accent {class}"
    );
    view! {
        <label class=class>
            <span class="size-[11px] shrink-0 border-[1.5px] border-ink-faint rounded-full"></span>
            <input
                class="grow min-w-0 bg-transparent outline-none text-[12.5px] text-ink placeholder:text-ink-faint"
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </label>
    }
}
