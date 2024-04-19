use leptos::*;

#[component]
pub fn Badge(children: Children) -> impl IntoView {
    view! {
        <span class="whitespace-nowrap rounded-full bg-indigo-700 px-3 py-1 text-indigo-100">
            {children()}
        </span>
    }
}
