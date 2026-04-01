use leptos::*;
use leptos_router::A;

#[component]
pub fn SidebarTop() -> impl IntoView {
    view! {
        <div class="flex items-center gap-3 px-4 py-5 border-b border-zinc-800">
            <img class="size-8" src="logo.png" />
            <span class="text-sm font-semibold text-white tracking-tight">"bld"</span>
        </div>
    }
}

#[component]
pub fn SidebarItem(
    #[prop(into)] icon: String,
    #[prop(into)] text: String,
    #[prop(into)] url: String,
) -> impl IntoView {
    view! {
        <A
            class="flex items-center gap-3 mx-2 px-3 py-2 rounded-lg text-sm font-medium text-zinc-400 hover:text-white hover:bg-zinc-800 aria-[current=page]:bg-violet-600/10 aria-[current=page]:text-violet-300 transition-colors duration-150"
            href=url
        >
            <i class=format!("{icon} text-base")></i>
            <span>{text}</span>
        </A>
    }
}

#[component]
pub fn SidebarBottom(children: Children) -> impl IntoView {
    view! {
        <div class="p-4 border-t border-zinc-800 flex flex-col gap-3">
            <div class="text-xs text-zinc-600 text-center">
                "Star on "
                <a
                    class="text-zinc-500 hover:text-violet-400 transition-colors"
                    target="_blank"
                    href="https://github.com/Kani-Maki-Gang/bld"
                >
                    "GitHub"
                </a> " · "
                <a
                    class="text-zinc-500 hover:text-violet-400 transition-colors"
                    target="_blank"
                    href="https://kani-maki-gang.github.io/bld-book/"
                >
                    "Docs"
                </a>
            </div>
            {children()}
        </div>
    }
}

#[component]
pub fn Sidebar(children: Children) -> impl IntoView {
    view! {
        <div class="bg-zinc-900 w-52 flex flex-col border-r border-zinc-800 shrink-0">
            {children()}
        </div>
    }
}
