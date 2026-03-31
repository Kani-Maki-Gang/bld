use leptos::*;

#[component]
pub fn Card(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!(
        "bg-zinc-900 rounded-xl flex flex-col gap-4 border border-zinc-800 shadow-xl shadow-black/30 {class}"
    );
    view! { <div class=class>{children()}</div> }
}
