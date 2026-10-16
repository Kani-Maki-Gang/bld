use crate::{
    api,
    components::card::{Card, Eyebrow},
    format,
};
use bld_models::dtos::{HistQueryParams, HistoryEntry};
use chrono::Datelike;
use leptos::*;
use leptos_router::A;

#[component]
fn EmptyValue(#[prop(into)] note: String) -> impl IntoView {
    view! {
        <div class="mt-3 text-[28px] font-semibold tracking-[-.03em] leading-none text-ink-muted">
            "—"
        </div>
        <div class="mt-2 text-ink-faint text-xs">{note}</div>
        <div class="mt-3.5 h-1.5 rounded-full bg-line-faint"></div>
    }
}

#[component]
fn RunningNow(#[prop(into)] runs: Vec<HistoryEntry>) -> impl IntoView {
    let Some(first) = runs.first().cloned() else {
        return view! {
            <Eyebrow>"Running now"</Eyebrow>
            <EmptyValue note="Nothing running" />
        }
        .into_view();
    };
    let elapsed = first
        .start_date_time
        .as_deref()
        .and_then(format::parse_date)
        .map(|start| format!("{} elapsed", format::clock(format::now() - start)))
        .unwrap_or_else(|| "starting".to_string());
    let more = runs.len().saturating_sub(1);
    let note = if more > 0 {
        format!("{elapsed} · {more} more running")
    } else {
        elapsed
    };
    view! {
        <div class="flex items-center justify-between gap-2.5">
            <Eyebrow>"Running now"</Eyebrow>
            <span class="size-1.5 rounded-full bg-run animate-bld-pulse"></span>
        </div>
        <A
            href=format!("/monit?id={}", first.id)
            class="block mt-3.5 font-mono text-[13px] truncate hover:text-accent-link"
        >
            {first.name.clone()}
        </A>
        <div class="mt-[5px] text-ink-muted text-xs">{note}</div>
        <div class="mt-3.5 h-1 rounded-full bg-[#1f232c] overflow-hidden relative">
            <div class="absolute inset-y-0 left-0 w-[30%] rounded-full bg-run animate-bld-sweep"></div>
        </div>
    }
    .into_view()
}

#[component]
pub fn DashboardStats(#[prop(into)] refresh: Signal<()>) -> impl IntoView {
    let running = create_resource(
        move || refresh.get(),
        |_| async move {
            let params = HistQueryParams {
                state: Some("running".to_string()),
                limit: 10,
                ..Default::default()
            };
            api::hist(params).await.ok()
        },
    );
    let queued = create_resource(
        move || refresh.get(),
        |_| async move { api::queued_pipelines().await.ok() },
    );
    let workers = create_resource(
        move || refresh.get(),
        |_| async move { api::running_pipelines().await.ok() },
    );
    let throughput = create_resource(
        move || refresh.get(),
        |_| async move { api::throughput().await.ok() },
    );
    let per_month = create_resource(
        move || refresh.get(),
        |_| async move { api::pipeline_runs_per_month().await.ok() },
    );

    let queue_view = move || {
        let queued = queued.get().flatten().map(|q| q.count).unwrap_or(0);
        let Some(workers) = workers.get().flatten() else {
            return view! {
                <Eyebrow>"Queue"</Eyebrow>
                <EmptyValue note="Worker data not available" />
            }
            .into_view();
        };
        let total = (workers.count + workers.available_workers).max(0) as usize;
        let busy = workers.count.max(0) as usize;
        let idle = workers.available_workers.max(0);
        let value = if queued == 0 {
            "Empty".to_string()
        } else {
            format!("{queued} waiting")
        };
        let note = if queued == 0 {
            format!("Nothing waiting. {idle} of {total} workers idle.")
        } else {
            format!("{idle} of {total} workers idle.")
        };
        view! {
            <Eyebrow>"Queue"</Eyebrow>
            <div class="mt-3 text-[28px] font-semibold tracking-[-.03em] leading-none">{value}</div>
            <div class="mt-2 text-ink-muted text-xs">{note}</div>
            <div class="mt-3.5 flex gap-[5px]">
                {(0..total)
                    .map(|i| {
                        let class = if i < busy { "bg-run" } else { "bg-line" };
                        view! { <div class=format!("flex-1 h-1.5 rounded-full {class}")></div> }
                    })
                    .collect_view()}
            </div>
        }
        .into_view()
    };

    let throughput_view = move || {
        let Some(kpi) = throughput.get().flatten() else {
            return view! {
                <Eyebrow>"Throughput"</Eyebrow>
                <EmptyValue note="No runs this month" />
            }
            .into_view();
        };
        let months = per_month.get().flatten().unwrap_or_default();
        let max = months.iter().map(|m| m.count).fold(0.0, f64::max);
        let current = chrono::Utc::now().month() as i64;
        view! {
            <Eyebrow>"Throughput"</Eyebrow>
            <div class="mt-3 flex items-baseline gap-2">
                <span class="text-[28px] font-semibold tracking-[-.03em] leading-none">
                    {kpi.total}
                </span>
                <span class="text-ink-muted text-[12.5px]">"runs this month"</span>
            </div>
            <div class="mt-2 text-ink-muted text-xs">
                {format!("{} from cron · {} manual", kpi.cron, kpi.manual)}
            </div>
            <div class="mt-3.5 flex gap-[2px] items-end h-[26px]">
                {months
                    .into_iter()
                    .map(|m| {
                        let height = if max > 0.0 { (m.count / max * 100.0).max(4.0) } else { 4.0 };
                        let color = if m.month == current {
                            "bg-accent"
                        } else if m.count > 0.0 {
                            "bg-accent/40"
                        } else {
                            "bg-line"
                        };
                        view! {
                            <div
                                class=format!("flex-1 rounded-[1.5px] {color}")
                                style=format!("height:{height}%")
                                title=format!("month {}: {} runs", m.month, m.count)
                            ></div>
                        }
                    })
                    .collect_view()}
            </div>
        }
        .into_view()
    };

    view! {
        <div class="mt-3.5 grid grid-cols-[repeat(auto-fit,minmax(260px,1fr))] gap-3.5">
            <Card class="px-5 py-[18px]">
                {move || {
                    running.get().map(|runs| view! { <RunningNow runs=runs.unwrap_or_default() /> })
                }}
            </Card>
            <Card class="px-5 py-[18px]">{queue_view}</Card>
            <Card class="px-5 py-[18px]">{throughput_view}</Card>
        </div>
    }
}
