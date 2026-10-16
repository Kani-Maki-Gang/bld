use leptos::*;

#[component]
pub fn Tabs(children: Children) -> impl IntoView {
    view! { <div class="mt-5 flex gap-[22px] border-b border-line">{children()}</div> }
}

#[component]
pub fn Tab(#[prop(into)] is_selected: Signal<bool>, children: Children) -> impl IntoView {
    let class = move || {
        if is_selected.get() {
            "h-9 -mb-px border-b-2 border-accent text-ink text-[13px] font-medium cursor-pointer"
        } else {
            "h-9 -mb-px border-b-2 border-transparent text-ink-muted hover:text-ink text-[13px] font-medium cursor-pointer"
        }
    };
    view! { <button class=class>{children()}</button> }
}
