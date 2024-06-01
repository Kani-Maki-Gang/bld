use leptos::*;

#[component]
pub fn ButtonGroup(children: Children) -> impl IntoView {
    view! {
        <div class="inline-flex rounded-lg border border-gray-800 p-1 bg-slate-900">
            {children()}
        </div>
    }
}

#[component]
pub fn ButtonGroupItem(
    #[prop(into, optional)] is_selected: Option<Signal<bool>>,
    children: Children
) -> impl IntoView {
    let class = move || if is_selected.as_ref().map(|x| x.get()).unwrap_or(false) {
        "inline-block rounded-md px-4 py-2 text-sm text-gray-200 shadow-sm focus:relative bg-slate-800"
    } else {
        "inline-block rounded-md px-4 py-2 text-sm text-gray-400 hover:text-gray-200 focus:relative"
    };
    view! {
        <button class=class>{children()}</button>
    }
}
