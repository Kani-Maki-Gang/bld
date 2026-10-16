use crate::{
    components::{
        badge::{RunState, TickStrip},
        card::Card,
        page::LoadingState,
    },
    error::ErrorBanner,
    pages::home::shared::{
        health_last, health_last_completed, health_rate, health_state, health_ticks, pipeline_href,
    },
};
use bld_models::dtos::PipelineHealthKpi;
use leptos::*;
use leptos_router::A;

type HealthResource = Resource<(), Result<Vec<PipelineHealthKpi>, String>>;

#[component]
fn Legend() -> impl IntoView {
    view! {
        <div class="flex items-center gap-3.5 text-[11.5px] text-ink-muted">
            <span class="flex items-center gap-1.5">
                <span class="size-2 rounded-sm bg-ok"></span>
                "success"
            </span>
            <span class="flex items-center gap-1.5">
                <span class="size-2 rounded-sm bg-bad"></span>
                "faulted"
            </span>
            <span class="flex items-center gap-1.5">
                <span class="size-2 rounded-sm bg-run"></span>
                "running"
            </span>
            <span class="flex items-center gap-1.5">
                <span class="size-2 rounded-sm bg-line"></span>
                "no run"
            </span>
        </div>
    }
}

#[component]
fn HealthRow(#[prop()] kpi: PipelineHealthKpi) -> impl IntoView {
    let state = health_state(&kpi);
    let rate_class = if health_last_completed(&kpi) == RunState::Faulted {
        "text-bad"
    } else {
        "text-ink-soft"
    };
    view! {
        <A
            href=pipeline_href(&kpi.id, &kpi.pipeline)
            class="flex items-center gap-[18px] px-5 py-[13px] border-t border-line-soft hover:bg-hover flex-wrap"
        >
            <div class="flex-1 min-w-[150px] flex items-center gap-[9px]">
                <span class=format!("size-1.5 rounded-full shrink-0 {}", state.dot_class())></span>
                <span class="font-mono text-[12.5px] truncate">{kpi.pipeline.clone()}</span>
            </div>
            <TickStrip
                states=health_ticks(&kpi)
                class="flex-[2] min-w-[140px]"
                tick_class="max-w-[7px] h-[18px]"
            />
            <div class=format!(
                "w-[120px] text-right font-mono text-xs shrink-0 {rate_class}",
            )>{health_rate(&kpi)}</div>
            <div class="w-[100px] text-right text-xs text-ink-muted shrink-0">
                {health_last(&kpi)}
            </div>
        </A>
    }
}

#[component]
pub fn DashboardHealth(#[prop()] health: HealthResource) -> impl IntoView {
    view! {
        <Card class="mt-3.5 overflow-hidden">
            <div class="px-5 pt-[18px] pb-3.5 flex items-end justify-between gap-4 flex-wrap">
                <div>
                    <div class="text-[15px] font-semibold tracking-[-.01em]">"Pipeline health"</div>
                    <div class="text-ink-muted text-[12.5px] mt-[3px]">
                        "Last 30 runs per pipeline, oldest on the left"
                    </div>
                </div>
                <Legend />
            </div>
            {move || match health.get() {
                None => view! { <LoadingState /> }.into_view(),
                Some(Err(e)) => {
                    view! {
                        <div class="px-5 pb-5">
                            <ErrorBanner error=e />
                        </div>
                    }
                        .into_view()
                }
                Some(Ok(rows)) if rows.is_empty() => {
                    let no_runs: Vec<RunState> = vec![];
                    view! {
                        <div class="border-t border-line-soft px-[22px] py-11 flex flex-col items-center gap-2.5 text-center">
                            <TickStrip
                                states=no_runs.clone()
                                class="opacity-55 w-[270px]"
                                tick_class="h-[18px] !bg-line-faint"
                            />
                            <div class="text-sm font-medium mt-1.5">"No runs to chart yet"</div>
                            <div class="text-[12.5px] text-ink-muted max-w-[340px] leading-[1.55]">
                                "Health strips fill in as runs complete. The first run appears here within seconds of starting."
                            </div>
                        </div>
                    }
                        .into_view()
                }
                Some(Ok(rows)) => {
                    rows.into_iter().map(|kpi| view! { <HealthRow kpi=kpi /> }).collect_view()
                }
            }}
        </Card>
    }
}
