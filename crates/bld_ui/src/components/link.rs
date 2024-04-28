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
pub fn LinkButton(#[prop(into)] href: Signal<String>, children: Children) -> impl IntoView {
    view! {
        <A href=move || href.get()
            class="h-[40px] text-center text-white rounded-lg w-full bg-indigo-600 p-2 hover:bg-indigo-700 focus:bg-indigo-700 focus:outline-none">
            {children()}
        </A>
    }
}
