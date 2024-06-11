use leptos::*;

#[derive(Clone, Default)]
pub enum TabsDirection {
    #[default]
    Horizontal,
    Vertical,
}

#[component]
pub fn Tabs(
    #[prop(into, optional)] direction: Signal<TabsDirection>,
    children: Children,
) -> impl IntoView {
    let nav_class = move || {
        direction.with(|d| match d {
            TabsDirection::Horizontal => "flex gap-6",
            TabsDirection::Vertical => "flex flex-col gap-6",
        })
    };
    view! {
        <div class="flex flex-col">
            <div class="hidden sm:block">
                <nav class=move || nav_class() aria-label="Tabs">
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
