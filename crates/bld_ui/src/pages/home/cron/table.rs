use super::delete::CronJobDeleteButton;
use crate::{
    api,
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        card::Card,
        link::LinkButton,
        page::{EmptyState, LoadingState},
    },
    context::{RefreshCronJobs, use_app_dialog},
    error::ErrorBanner,
    format,
};
use bld_models::dtos::{CronJobResponse, JobFiltersParams};
use leptos::*;
use leptos_router::A;
use leptos_use::signal_debounced;

const GRID: &str = "grid grid-cols-[minmax(0,1fr)_150px_150px_110px_132px_150px] gap-3.5 px-5";

#[component]
fn CronHead() -> impl IntoView {
    view! {
        <div class=format!(
            "{GRID} py-[11px] border-b border-line font-mono text-[10.5px] tracking-[.09em] uppercase text-ink-faint",
        )>
            <div>"Pipeline"</div>
            <div>"Schedule"</div>
            <div>"Cadence"</div>
            <div>"Next run"</div>
            <div>"Created"</div>
            <div class="text-right">"Actions"</div>
        </div>
    }
}

#[component]
fn CronRow(#[prop()] job: CronJobResponse) -> impl IntoView {
    let dialog = use_app_dialog();
    let refresh = use_context::<RefreshCronJobs>();
    let toggle = create_action(move |(id, enabled): &(String, bool)| {
        let (id, enabled) = (id.clone(), *enabled);
        async move {
            match api::cron_set_enabled(id, enabled).await {
                Ok(_) => {
                    if let Some(refresh) = refresh {
                        refresh.set();
                    }
                }
                Err(e) => dialog.error(e.to_string()),
            }
        }
    });

    let (next, next_class) = if !job.is_enabled {
        ("paused".to_string(), "text-warn")
    } else {
        match format::cron_next(&job.schedule) {
            Some(date) => (format::until(date), "text-ink-soft"),
            None => ("invalid".to_string(), "text-bad"),
        }
    };
    let row_class = if job.is_enabled { "" } else { "opacity-70" };
    let id = job.id.clone();
    let enabled = job.is_enabled;

    view! {
        <div class=format!("{GRID} py-[13px] border-b border-line-faint items-center {row_class}")>
            <div class="min-w-0">
                <A
                    href=format!("/files/info?name={}", job.pipeline)
                    class="block font-mono text-[12.5px] truncate hover:text-accent-link"
                >
                    {job.pipeline.clone()}
                </A>
                <div class="font-mono text-[10.5px] text-ink-faint mt-[3px] truncate">
                    {job.id.clone()} {job.is_default.then_some(" · default")}
                </div>
            </div>
            <div class="min-w-0">
                <span class="font-mono text-[11.5px] bg-chip border border-line-strong rounded-md px-[7px] py-[3px] whitespace-nowrap">
                    {job.schedule.clone()}
                </span>
            </div>
            <div class="text-[12.5px] text-ink-soft truncate">
                {format::cron_cadence(&job.schedule)}
            </div>
            <div class=format!("font-mono text-xs {next_class}")>{next}</div>
            <div class="font-mono text-xs text-ink-muted">
                {format::short_date(&job.date_created)}
            </div>
            <div class="flex gap-1.5 justify-end">
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Xs
                    on:click=move |_| toggle.dispatch((id.clone(), !enabled))
                >
                    {if enabled { "Pause" } else { "Resume" }}
                </Button>
                <LinkButton
                    href=format!("/cron/update?id={}", job.id)
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Xs
                >
                    "Edit"
                </LinkButton>
                <CronJobDeleteButton id=job.id.clone() />
            </div>
        </div>
    }
}

#[component]
pub fn CronJobsTable(#[prop(into)] search: Signal<String>) -> impl IntoView {
    let refresh = use_context::<RefreshCronJobs>();
    let search = signal_debounced(search, 300.0);

    let data = create_resource(
        move || refresh.map(|r| r.0.get()),
        |_| async move {
            api::cron(JobFiltersParams::default())
                .await
                .map_err(|e| e.to_string())
        },
    );

    view! {
        <Card class="mt-[18px] overflow-hidden">
            <div class="overflow-x-auto">
                <div class="min-w-[900px]">
                    <CronHead />
                    {move || match data.get() {
                        None => view! { <LoadingState /> }.into_view(),
                        Some(Err(e)) => {
                            view! {
                                <div class="p-5">
                                    <ErrorBanner error=e />
                                </div>
                            }
                                .into_view()
                        }
                        Some(Ok(jobs)) if jobs.is_empty() => {
                            view! {
                                <EmptyState
                                    title="Nothing scheduled"
                                    body="Attach a cron expression to a pipeline to have the server run it unattended."
                                    visual=view! {
                                        <span class="font-mono text-xs text-ink-muted bg-hover border border-dashed border-line-edge rounded-[7px] px-[9px] py-[5px]">
                                            "0 */10 * * * *"
                                        </span>
                                    }
                                        .into_view()
                                />
                            }
                                .into_view()
                        }
                        Some(Ok(jobs)) => {
                            let search = search.get();
                            let jobs: Vec<_> = jobs
                                .into_iter()
                                .filter(|j| search.is_empty() || j.pipeline.contains(&search))
                                .collect();
                            if jobs.is_empty() {
                                return view! {
                                    <div class="px-5 py-10 text-center text-[12.5px] text-ink-muted">
                                        "No cron jobs match this filter."
                                    </div>
                                }
                                    .into_view();
                            }
                            jobs.into_iter().map(|job| view! { <CronRow job=job /> }).collect_view()
                        }
                    }}
                </div>
            </div>
        </Card>
    }
}
