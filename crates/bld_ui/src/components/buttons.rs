use leptos::*;

#[component]
pub fn PrimaryBtn(children: Children) -> impl IntoView {
    view! {
        <button class="bg-primary">{children()}</button>
    }
}

#[component]
pub fn AccentBtn(children: Children) -> impl IntoView {
    view! {
        <button class="bg-accent">{children()}</button>
    }
}

#[component]
pub fn AccentLightBtn(children: Children) -> impl IntoView {
    view! {
        <button class="bg-accent-light">{children()}</button>
    }
}
