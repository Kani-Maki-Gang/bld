use crate::{
    components::{
        badge::{Chip, RunState},
        button::{Button, ButtonSize, ButtonVariant},
        link::LinkButton,
        page::{EmptyState, LoadingState},
    },
    error::ErrorBanner,
    format,
};
use bld_models::dtos::HistoryEntry;
use leptos::*;
use leptos_router::A;

const GRID: &str =
    "grid grid-cols-[104px_minmax(0,1.1fr)_minmax(0,1.2fr)_110px_150px_84px_38px] gap-3.5 px-5";

type RowsResource =
    Resource<((Option<String>, Option<String>), u64), Result<Vec<HistoryEntry>, String>>;

#[component]
pub fn HistoryHead() -> impl IntoView {
    view! {
        <div class=format!(
            "{GRID} py-[11px] border-b border-line font-mono text-[10.5px] tracking-[.09em] uppercase text-ink-faint",
        )>
            <div>"State"</div>
            <div>"Run"</div>
            <div>"Pipeline"</div>
            <div>"Trigger"</div>
            <div>"Started"</div>
            <div>"Duration"</div>
            <div></div>
        </div>
    }
}

#[component]
pub fn HistoryRow(#[prop()] entry: HistoryEntry) -> impl IntoView {
    let state = RunState::parse(&entry.state);
    let duration = format::run_duration(
        entry.start_date_time.as_deref(),
        entry.end_date_time.as_deref(),
    );
    view! {
        <A
            href=format!("/monit?id={}", entry.id)
            class=format!("{GRID} py-[11px] border-b border-line-faint items-center hover:bg-hover")
        >
            <div class=format!("flex items-center gap-[7px] text-xs {}", state.text_class())>
                <span class=format!("size-1.5 rounded-full shrink-0 {}", state.dot_class())></span>
                <span>{state.label()}</span>
            </div>
            <div class="font-mono text-xs text-ink-soft truncate" title=entry.id.clone()>
                {entry.id.clone()}
            </div>
            <div class="font-mono text-xs truncate">{entry.name.clone()}</div>
            <div class="min-w-0">
                <Chip class="max-w-full truncate">{format::trigger_label(&entry.user)}</Chip>
            </div>
            <div class="font-mono text-xs text-ink-muted">
                {entry.start_date_time.clone().unwrap_or_else(|| "—".to_string())}
            </div>
            <div class="font-mono text-xs text-ink-muted">{duration}</div>
            <div class="text-right text-ink-faint text-sm">"›"</div>
        </A>
    }
}

#[component]
pub fn HistoryRows<F>(
    #[prop()] rows: RowsResource,
    #[prop(into)] has_filters: Signal<bool>,
    #[prop(into)] total: Signal<u64>,
    on_clear: F,
) -> impl IntoView
where
    F: Fn() + Copy + 'static,
{
    move || {
        match rows.get() {
        None => view! { <LoadingState /> }.into_view(),
        Some(Err(e)) => view! {
            <div class="p-5">
                <ErrorBanner error=e />
            </div>
        }.into_view(),
        Some(Ok(rows)) if rows.is_empty() && has_filters.get() => {
            let body = format!(
                "{} exist, but none of them match the current search and state filter.",
                crate::pages::home::shared::plural(total.get(), "run", "runs")
            );
            view! {
                <EmptyState title="No runs match this filter" body=body>
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::Sm
                        on:click=move |_| on_clear()
                    >
                        "Clear filters"
                    </Button>
                </EmptyState>
            }
            .into_view()
        }
        Some(Ok(rows)) if rows.is_empty() => {
            view! {
                <EmptyState
                    title="No runs yet"
                    body="Runs land here as soon as a pipeline is triggered manually, by cron, or over the API."
                    visual=view! {
                        <div class="size-[26px] rounded-lg border border-dashed border-[#323744]"></div>
                    }
                        .into_view()
                >
                    <LinkButton
                        href="/files"
                        variant=ButtonVariant::Secondary
                        size=ButtonSize::Sm
                    >
                        "Run a pipeline"
                    </LinkButton>
                </EmptyState>
            }
            .into_view()
        }
        Some(Ok(rows)) => rows
            .into_iter()
            .map(|entry| view! { <HistoryRow entry=entry /> })
            .collect_view(),
    }
    }
}
