use super::{button::get_button_color_classes, colors::Colors};
use leptos::*;
use leptos_router::A;

#[component]
pub fn Link(#[prop(into)] href: String, children: Children) -> impl IntoView {
    view! {
        <A class="text-blue-400 underline"
            href=href>
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
    let class =
        format!("h-[40px] w-full text-center rounded-lg p-2 focus:outline-none {color} {class}");
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
        "h-[40px] w-[40px] text-xl text-center rounded-lg p-2 focus:outline-none {color} {class}"
    );
    view! {
        <A href=move || href.get() class=class>
            <i class=icon></i>
        </A>
    }
}
