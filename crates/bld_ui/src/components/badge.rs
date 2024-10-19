use leptos::*;

#[component]
pub fn Badge(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!(
        "whitespace-nowrap rounded-full bg-indigo-700 px-3 py-1 text-indigo-100 w-fit {class}"
    );
    view! { <div class=class>{children()}</div> }
}
