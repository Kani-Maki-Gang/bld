use leptos::*;

#[component]
pub fn Tabs(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-col">
            <div class="hidden sm:block">
                <nav class="flex gap-6" aria-label="Tabs">
                    {children()}
                </nav>
            </div>
        </div>
    }
}

#[component]
pub fn Tab(#[prop(into)] is_selected: Signal<bool>, children: Children) -> impl IntoView {
    let class = move || {
        if is_selected.get() {
            "shrink-0 rounded-lg px-4 py-2 text-sm font-medium text-gray-200 bg-slate-800"
        } else {
            "shrink-0 rounded-lg px-4 py-2 text-sm font-medium text-gray-400 hover:text-gray-200"
        }
    };
    view! {
        <button class=class>
            {children()}
        </button>
    }
}
