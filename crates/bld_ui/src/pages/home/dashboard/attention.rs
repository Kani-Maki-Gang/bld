use crate::{
    api,
    components::{
        badge::TickStrip,
        button::{Button, ButtonSize, ButtonVariant},
        link::LinkButton,
    },
    error::ErrorBanner,
    format,
    pages::home::shared::{health_ticks, plural},
};
use anyhow::Result;
use bld_models::dtos::{JobFiltersParams, PipelineHealthKpi};
use leptos::*;

type HealthResource = Resource<(), Result<Vec<PipelineHealthKpi>, String>>;

async fn pause_schedules(pipeline: String) -> Result<u64> {
    let params = JobFiltersParams {
        pipeline: Some(pipeline),
        ..Default::default()
    };
    let mut paused = 0;
    for job in api::cron(params).await? {
        if job.is_enabled {
            api::cron_set_enabled(job.id, false).await?;
            paused += 1;
        }
    }
    Ok(paused)
}

#[component]
fn FailingRow(#[prop()] kpi: PipelineHealthKpi) -> impl IntoView {
    let message = create_rw_signal(None::<String>);
    let pause = create_action(move |pipeline: &String| {
        let pipeline = pipeline.clone();
        async move {
            let text = match pause_schedules(pipeline).await {
                Ok(0) => "No active schedules".to_string(),
                Ok(n) => format!("Paused {}", plural(n, "schedule", "schedules")),
                Err(e) => e.to_string(),
            };
            message.set(Some(text));
        }
    });
    let ticks = health_ticks(&kpi);
    let last_faulted = kpi
        .runs
        .iter()
        .rev()
        .find(|r| r.state == "faulted")
        .map(|r| r.id.clone())
        .or_else(|| kpi.last_run_id.clone())
        .unwrap_or_default();
    let last_log = format!("/monit?id={last_faulted}");
    let name = kpi.pipeline.clone();

    view! {
        <div class="flex items-center gap-4 flex-wrap pt-4 border-t border-[#2a2b33]">
            <div class="flex items-center gap-2.5 min-w-0">
                <span class="size-6 rounded-[7px] bg-chip border border-line-edge grid place-items-center font-mono text-[10px] text-ink-muted">
                    "y"
                </span>
                <span class="font-mono text-[13px] truncate">{kpi.pipeline.clone()}</span>
                <span class="text-xs text-ink-muted whitespace-nowrap">
                    {plural(kpi.faulted_streak, "faulted run", "faulted runs")}
                </span>
            </div>
            <TickStrip states=ticks class="flex-1 min-w-[120px]" tick_class="max-w-[8px] h-5" />
            <div class="flex items-center gap-2 shrink-0">
                <Show when=move || message.get().is_some() fallback=|| view! {}>
                    <span class="text-xs text-ink-muted">{move || message.get()}</span>
                </Show>
                <LinkButton href=last_log variant=ButtonVariant::Secondary size=ButtonSize::Sm>
                    "Open last log"
                </LinkButton>
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Sm
                    on:click=move |_| pause.dispatch(name.clone())
                >
                    "Pause schedule"
                </Button>
            </div>
        </div>
    }
}

#[component]
pub fn DashboardAttention(#[prop()] health: HealthResource) -> impl IntoView {
    move || match health.get() {
        None => view! {}.into_view(),
        Some(Err(e)) => view! {
            <div class="mt-5">
                <ErrorBanner error=e />
            </div>
        }
        .into_view(),
        Some(Ok(health)) => {
            let mut failing: Vec<PipelineHealthKpi> = health
                .iter()
                .filter(|h| h.faulted_streak > 0)
                .cloned()
                .collect();
            failing.sort_by_key(|h| std::cmp::Reverse(h.faulted_streak));
            let has_runs = health.iter().any(|h| !h.runs.is_empty());

            if failing.is_empty() {
                let (body, empty) = if has_runs {
                    (
                        "The latest completed run of every pipeline succeeded.",
                        false,
                    )
                } else {
                    ("No runs recorded yet. Add a pipeline to get started.", true)
                };
                return view! {
                    <div class="mt-5 border border-line bg-surface rounded-[14px] px-[22px] py-[22px]">
                        <div class="flex items-center gap-2.5">
                            <span class="size-[7px] rounded-full bg-ok"></span>
                            <span class="font-mono text-[11px] font-semibold tracking-[.09em] text-ok uppercase">
                                "All clear"
                            </span>
                        </div>
                        <div class="mt-3 flex items-baseline gap-3 flex-wrap">
                            <div class="text-[32px] font-semibold tracking-[-.03em] leading-none">
                                "Nothing needs attention"
                            </div>
                            <div class="text-ink-muted text-[13px]">{body}</div>
                        </div>
                        <Show when=move || empty fallback=|| view! {}>
                            <div class="mt-[18px] pt-4 border-t border-line flex gap-2 flex-wrap">
                                <LinkButton href="/files" variant=ButtonVariant::Primary>
                                    "Add a pipeline"
                                </LinkButton>
                                <a
                                    href="https://kani-maki-gang.github.io/bld-book/"
                                    target="_blank"
                                    class=crate::components::button::button_classes(
                                        ButtonVariant::Ghost,
                                        ButtonSize::Md,
                                    )
                                >
                                    "Read the docs"
                                </a>
                            </div>
                        </Show>
                    </div>
                }
                .into_view();
            }

            let headline = format!(
                "{} failing",
                plural(failing.len() as u64, "pipeline", "pipelines")
            );
            let first = &failing[0];
            let detail = match &first.faulted_streak_start {
                Some(start) if failing.len() == 1 => format!(
                    "{} in a row, first at {}",
                    plural(first.faulted_streak, "faulted run", "faulted runs"),
                    format::short_date(start)
                ),
                _ => format!(
                    "Longest streak: {} on {}",
                    plural(first.faulted_streak, "faulted run", "faulted runs"),
                    first.pipeline
                ),
            };

            view! {
                <div class="mt-5 border border-[#3a2a30] bg-gradient-to-b from-bad/[.07] to-bad/[.02] rounded-[14px] px-[22px] py-5">
                    <div class="flex items-center gap-2.5">
                        <span class="size-[7px] rounded-full bg-bad"></span>
                        <span class="font-mono text-[11px] font-semibold tracking-[.09em] text-bad uppercase">
                            "Needs attention"
                        </span>
                    </div>
                    <div class="mt-3 flex items-baseline gap-3 flex-wrap">
                        <div class="text-[32px] font-semibold tracking-[-.03em] leading-none">
                            {headline}
                        </div>
                        <div class="text-ink-muted text-[13px]">{detail}</div>
                    </div>
                    <div class="mt-[18px] flex flex-col gap-4">
                        {failing
                            .into_iter()
                            .map(|kpi| view! { <FailingRow kpi=kpi /> })
                            .collect_view()}
                    </div>
                </div>
            }
            .into_view()
        }
    }
}
