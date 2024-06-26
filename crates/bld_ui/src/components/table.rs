use leptos::*;

#[component]
pub fn Table(children: Children) -> impl IntoView {
    view! {
        <div class="overflow-auto overscroll-auto">
            <table class="min-w-full bg-slate-700 text-sm">{children()}</table>
        </div>
    }
}

#[component]
pub fn Headers(children: Children) -> impl IntoView {
    view! {
        <thead>
            <tr>{children()}</tr>
        </thead>
    }
}

#[component]
pub fn Header(children: Children) -> impl IntoView {
    view! {
        <th class="border border-b-4 border-slate-600 whitespace-nowrap p-4 font-bold text-left">
            {children()}
        </th>
    }
}

#[component]
pub fn Body(children: Children) -> impl IntoView {
    view! { <tbody>{children()}</tbody> }
}

#[component]
pub fn Row(children: Children) -> impl IntoView {
    view! { <tr>{children()}</tr> }
}

#[component]
pub fn Cell(children: Children) -> impl IntoView {
    view! { <td class="border border-slate-600 whitespace-nowrap p-4 text-left">{children()}</td> }
}
