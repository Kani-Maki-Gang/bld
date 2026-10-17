use super::button::{ButtonSize, ButtonVariant, button_classes};
use leptos::*;
use leptos_router::A;

#[component]
pub fn LinkButton(
    #[prop(into)] href: MaybeSignal<String>,
    #[prop(into, optional)] variant: ButtonVariant,
    #[prop(into, optional)] size: ButtonSize,
    #[prop(into, optional)] class: String,
    children: Children,
) -> impl IntoView {
    let class = format!("{} {class}", button_classes(variant, size));
    view! {
        <A href=move || href.get() class=class>
            {children()}
        </A>
    }
}

#[component]
pub fn BackLink(#[prop(into)] href: MaybeSignal<String>, children: Children) -> impl IntoView {
    view! {
        <A
            href=move || href.get()
            class="inline-block mb-1.5 text-xs text-ink-muted hover:text-ink transition-colors"
        >
            "‹ "
            {children()}
        </A>
    }
}
