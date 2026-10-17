use super::{badge::Dot, page::LogoMark};
use crate::api;
use leptos::*;
use leptos_router::{A, NavigateOptions, use_navigate};
use leptos_use::use_interval_fn;

#[component]
fn TopbarItem(#[prop(into)] text: String, #[prop(into)] url: String) -> impl IntoView {
    view! {
        <A
            class="h-[30px] inline-flex items-center px-3 rounded-lg text-[13px] font-medium text-ink-muted hover:text-ink aria-[current=page]:bg-accent/[.14] aria-[current=page]:text-accent-ink transition-colors"
            href=url
        >
            {text}
        </A>
    }
}

#[component]
fn WorkersChip() -> impl IntoView {
    let data = create_resource(
        || (),
        |_| async move { api::running_pipelines().await.ok() },
    );
    let _ = use_interval_fn(move || data.refetch(), 10_000);

    let label = move || match data.get().flatten() {
        Some(kpi) => format!(
            "{}/{} workers",
            kpi.count,
            kpi.count + kpi.available_workers
        ),
        None => "workers".to_string(),
    };
    let dot = move || match data.get().flatten() {
        Some(kpi) if kpi.count > 0 => "size-[5px] bg-run animate-bld-pulse".to_string(),
        Some(_) => "size-[5px] bg-ok".to_string(),
        None => "size-[5px] bg-ink-ghost".to_string(),
    };

    view! {
        <div class="hidden sm:flex items-center gap-[7px] px-2.5 h-7 border border-line-strong rounded-full font-mono text-[11px] text-ink-muted">
            <Dot class=Signal::derive(dot) />
            <span>{label}</span>
        </div>
    }
}

#[component]
pub fn Topbar() -> impl IntoView {
    view! {
        <header class="sticky top-0 z-30 bg-canvas/[.86] backdrop-blur-md border-b border-line-soft">
            <div class="max-w-[1480px] mx-auto px-[22px] h-[54px] flex items-center gap-[26px]">
                <A href="/" class="flex items-center gap-[9px] shrink-0">
                    <LogoMark />
                    <span class="font-mono text-sm font-semibold tracking-[-.01em]">"bld"</span>
                </A>
                <nav class="flex items-center gap-[2px] grow min-w-0 overflow-x-auto">
                    <TopbarItem text="Overview" url="/dashboard" />
                    <TopbarItem text="History" url="/history" />
                    <TopbarItem text="Files" url="/files" />
                    <TopbarItem text="Cron jobs" url="/cron" />
                </nav>
                <div class="flex items-center gap-3.5 shrink-0">
                    <WorkersChip />
                    <button
                        class="h-[30px] px-3 border border-line-strong rounded-lg bg-transparent text-ink-soft text-[12.5px] hover:text-ink hover:border-line-bright transition-colors"
                        on:click=move |_| {
                            if let Err(e) = api::remove_auth_tokens() {
                                logging::error!("{e}");
                            }
                            let nav = use_navigate();
                            nav("/login", NavigateOptions::default());
                        }
                    >
                        "Logout"
                    </button>
                </div>
            </div>
        </header>
    }
}
