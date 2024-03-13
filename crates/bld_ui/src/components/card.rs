use leptos::*;

#[component]
pub fn CardHeader(children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-800 p-12">{children()}</div>
    }
}

#[component]
pub fn CardBody(children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-700 p-12">{children()}</div>
    }
}

#[component]
pub fn CardFooter(children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-700 p-12">{children()}</div>
    }
}

#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-700 rounded-xl">{children()}</div>
    }
}
