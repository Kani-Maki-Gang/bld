use leptos::*;

#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-700 rounded-xl">{children()}</div>
    }
}
