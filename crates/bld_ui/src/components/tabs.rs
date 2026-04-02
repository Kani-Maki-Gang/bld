use leptos::*;

#[derive(Clone, Default)]
pub enum TabsDirection {
    #[default]
    Horizontal,
    #[allow(unused)]
    Vertical,
}

#[component]
pub fn Tabs(
    #[prop(into, optional)] direction: Signal<TabsDirection>,
    children: Children,
) -> impl IntoView {
    let nav_class = move || {
        direction.with(|d| match d {
            TabsDirection::Horizontal => "flex gap-1 p-1 bg-zinc-800/50 rounded-lg w-fit",
            TabsDirection::Vertical => "flex flex-col gap-1 p-1 bg-zinc-800/50 rounded-lg",
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
            "shrink-0 rounded-md px-4 py-1.5 text-sm font-medium text-white bg-zinc-700 shadow-sm transition-colors duration-150"
        } else {
            "shrink-0 rounded-md px-4 py-1.5 text-sm font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700/50 transition-colors duration-150"
        }
    };
    view! { <button class=class>{children()}</button> }
}
