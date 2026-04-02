use leptos::*;

#[component]
pub fn Badge(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!(
        "whitespace-nowrap rounded-full bg-violet-500/10 border border-violet-500/20 px-2.5 py-0.5 text-xs font-medium text-violet-300 w-fit {class}"
    );
    view! { <div class=class>{children()}</div> }
}
