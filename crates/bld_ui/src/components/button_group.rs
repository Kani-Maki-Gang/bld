use leptos::*;

#[component]
pub fn ButtonGroup(children: Children) -> impl IntoView {
    view! {
        <div class="inline-flex rounded-lg border border-zinc-700 p-0.5 bg-zinc-900">
            {children()}
        </div>
    }
}

#[component]
pub fn ButtonGroupItem(
    #[prop(into, optional)] is_selected: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let class = move || {
        if is_selected.get() {
            "inline-block rounded-md px-4 py-1.5 text-sm font-medium text-white shadow-sm bg-zinc-700 transition-colors duration-150"
        } else {
            "inline-block rounded-md px-4 py-1.5 text-sm font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors duration-150"
        }
    };
    view! { <button class=class>{children()}</button> }
}
