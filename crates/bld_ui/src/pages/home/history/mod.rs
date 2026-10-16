mod table;

use crate::{
    api,
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        card::Card,
        input::SearchInput,
        page::PageHeader,
    },
};
use bld_models::dtos::{HistCountQueryParams, HistQueryParams};
use leptos::*;
use leptos_use::signal_debounced;
use table::{HistoryHead, HistoryRows};

const PAGE_SIZE: u64 = 25;

const FILTERS: [(&str, Option<&str>); 5] = [
    ("All", None),
    ("Faulted", Some("faulted")),
    ("Running", Some("running")),
    ("Queued", Some("queued")),
    ("Success", Some("finished")),
];

#[component]
pub fn History() -> impl IntoView {
    let search = create_rw_signal(String::new());
    let state = create_rw_signal(None::<&'static str>);
    let page = create_rw_signal(0u64);
    let search_debounced = signal_debounced(search, 400.0);

    let filters = move || {
        let search = search_debounced.get();
        (
            state.get().map(|s| s.to_string()),
            (!search.is_empty()).then_some(search),
        )
    };

    create_effect(move |_| {
        filters();
        page.set(0);
    });

    let rows = create_resource(
        move || (filters(), page.get()),
        |((state, search), page)| async move {
            let params = HistQueryParams {
                state,
                search,
                limit: PAGE_SIZE,
                offset: Some(page * PAGE_SIZE),
                ..Default::default()
            };
            api::hist(params)
                .await
                .map(|mut rows| {
                    rows.reverse();
                    rows
                })
                .map_err(|e| e.to_string())
        },
    );

    let count = create_resource(filters, |(state, search)| async move {
        let params = HistCountQueryParams {
            state,
            search,
            ..Default::default()
        };
        api::hist_count(params).await.unwrap_or(0)
    });

    let total = create_resource(
        || (),
        |_| async move {
            api::hist_count(HistCountQueryParams::default())
                .await
                .unwrap_or(0)
        },
    );

    let has_filters = Signal::derive(move || filters() != (None, None));
    let shown = move || {
        rows.get()
            .and_then(|r| r.ok())
            .map(|r| r.len() as u64)
            .unwrap_or(0)
    };
    let footer = move || {
        let count = count.get().unwrap_or(0);
        let start = page.get() * PAGE_SIZE;
        if shown() == 0 {
            format!("Showing 0 of {count} runs")
        } else {
            format!("Showing {}-{} of {count} runs", start + 1, start + shown())
        }
    };
    let has_newer = move || page.get() > 0;
    let has_older = move || (page.get() + 1) * PAGE_SIZE < count.get().unwrap_or(0);

    view! {
        <PageHeader title="History" subtitle="Pipeline runs ordered by start date, newest first">
            <SearchInput
                placeholder="Search runs, pipelines, users…"
                value=search
                class="min-w-[230px]"
            />
            {FILTERS
                .into_iter()
                .map(|(label, value)| {
                    let class = move || {
                        if state.get() == value {
                            "h-8 px-[11px] rounded-[9px] border border-accent-line bg-accent/[.14] text-accent-ink text-[12.5px] font-medium"
                        } else {
                            "h-8 px-[11px] rounded-[9px] border border-line-strong bg-transparent text-ink-muted hover:text-ink text-[12.5px] font-medium"
                        }
                    };
                    view! {
                        <button class=class on:click=move |_| state.set(value)>
                            {label}
                        </button>
                    }
                })
                .collect_view()}
        </PageHeader>
        <Card class="mt-[18px] overflow-hidden">
            <div class="overflow-x-auto">
                <div class="min-w-[860px]">
                    <HistoryHead />
                    <HistoryRows
                        rows=rows
                        has_filters=has_filters
                        total=Signal::derive(move || total.get().unwrap_or(0))
                        on_clear=move || {
                            search.set(String::new());
                            state.set(None);
                        }
                    />
                </div>
            </div>
            <Show when=move || { shown() > 0 || page.get() > 0 } fallback=|| view! {}>
                <div class="px-5 py-[13px] flex items-center justify-between gap-3 flex-wrap border-t border-line-faint">
                    <span class="text-xs text-ink-faint">{footer}</span>
                    <div class="flex gap-1.5">
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::Sm
                            class="disabled:text-ink-faint disabled:hover:border-line-strong"
                            disabled=Signal::derive(move || !has_newer())
                            on:click=move |_| page.update(|p| *p = p.saturating_sub(1))
                        >
                            "Newer"
                        </Button>
                        <Button
                            variant=ButtonVariant::Secondary
                            size=ButtonSize::Sm
                            class="disabled:bg-transparent disabled:text-ink-faint disabled:border-line-strong"
                            disabled=Signal::derive(move || !has_older())
                            on:click=move |_| page.update(|p| *p += 1)
                        >
                            "Older"
                        </Button>
                    </div>
                </div>
            </Show>
        </Card>
    }
}
