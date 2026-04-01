use leptos::*;
use leptos_router::A;

#[component]
pub fn TopbarItem(
    #[prop(into)] icon: String,
    #[prop(into)] text: String,
    #[prop(into)] url: String,
) -> impl IntoView {
    view! {
        <A
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium text-zinc-400 hover:text-white hover:bg-zinc-800 aria-[current=page]:bg-violet-600/10 aria-[current=page]:text-violet-300 transition-colors duration-150"
            href=url
        >
            <i class=format!("{icon} text-base")></i>
            <span>{text}</span>
        </A>
    }
}

#[component]
pub fn Topbar(children: Children) -> impl IntoView {
    view! {
        <header class="h-14 bg-zinc-900 border-b border-zinc-800 flex items-center px-4 gap-2 shrink-0">
            <div class="flex items-center gap-2.5 mr-6">
                <img class="size-8" src="logo_no_bg.png" />
                <span class="text-sm font-semibold text-white tracking-tight">"bld"</span>
            </div>
            <nav class="flex items-center gap-1 grow">
                <TopbarItem icon="iconoir-presentation" text="Dashboard" url="/" />
                <TopbarItem icon="iconoir-book" text="History" url="/history" />
                <TopbarItem icon="iconoir-wrench" text="Pipelines" url="/pipelines" />
                <TopbarItem icon="iconoir-clock-rotate-right" text="Cron jobs" url="/cron" />
            </nav>
            <div class="flex items-center gap-2">{children()}</div>
        </header>
    }
}
