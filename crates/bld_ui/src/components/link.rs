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
