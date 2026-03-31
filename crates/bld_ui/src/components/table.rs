use leptos::*;

#[component]
pub fn Table(children: Children) -> impl IntoView {
    view! {
        <div class="overflow-auto overscroll-auto rounded-lg border border-zinc-800">
            <table class="min-w-full bg-zinc-900 text-sm">{children()}</table>
        </div>
    }
}

#[component]
pub fn Headers(children: Children) -> impl IntoView {
    view! {
        <thead class="bg-zinc-950/60">
            <tr>{children()}</tr>
        </thead>
    }
}

#[component]
pub fn Header(children: Children) -> impl IntoView {
    view! {
        <th class="border-b border-zinc-800 whitespace-nowrap px-4 py-3 text-xs font-semibold text-zinc-400 uppercase tracking-wider text-left">
            {children()}
        </th>
    }
}

#[component]
pub fn Body(children: Children) -> impl IntoView {
    view! { <tbody class="divide-y divide-zinc-800/60">{children()}</tbody> }
}

#[component]
pub fn Row(children: Children) -> impl IntoView {
    view! { <tr class="hover:bg-zinc-800/40 transition-colors duration-100">{children()}</tr> }
}

#[component]
pub fn Cell(children: Children) -> impl IntoView {
    view! {
        <td class="whitespace-nowrap px-4 py-3 text-left text-zinc-200">{children()}</td>
    }
}
