use leptos::*;

#[component]
pub fn Card(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class =
        format!("bg-slate-700 rounded-xl flex flex-col gap-4 border border-slate-600 {class}");
    view! { <div class=class>{children()}</div> }
}
