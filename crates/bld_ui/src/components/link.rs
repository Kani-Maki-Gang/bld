use super::{button::get_button_color_classes, colors::Colors};
use leptos::*;
use leptos_router::A;

#[component]
pub fn Link(#[prop(into)] href: String, children: Children) -> impl IntoView {
    view! {
        <A class="text-violet-400 hover:text-violet-300 underline underline-offset-2 transition-colors duration-150" href=href>
            {children()}
        </A>
    }
}

#[component]
pub fn LinkButton(
    #[prop(into)] href: Signal<String>,
    #[prop(into, optional)] color: Option<Colors>,
    #[prop(into, optional)] class: String,
    children: Children,
) -> impl IntoView {
    let color = get_button_color_classes(color);
    let class = format!(
        "h-[38px] w-full text-center text-sm font-medium rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-violet-500/40 transition-colors duration-150 {color} {class}"
    );
    view! {
        <A href=move || href.get() class=class>
            {children()}
        </A>
    }
}

#[component]
pub fn LinkIconButton(
    #[prop(into)] href: Signal<String>,
    #[prop(into)] icon: String,
    #[prop(into, optional)] color: Option<Colors>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let color = get_button_color_classes(color);
    let class = format!(
        "h-[38px] w-[38px] text-base flex items-center justify-center rounded-lg focus:outline-none focus:ring-2 focus:ring-violet-500/40 transition-colors duration-150 {color} {class}"
    );
    view! {
        <A href=move || href.get() class=class>
            <i class=icon></i>
        </A>
    }
}
