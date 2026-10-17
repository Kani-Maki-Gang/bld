use crate::{
    api,
    components::{
        button::{ButtonSize, ButtonVariant},
        card::Card,
        link::LinkButton,
        page::{EmptyState, LoadingState},
    },
    error::ErrorBanner,
    format,
};
use bld_models::dtos::JobFiltersParams;
use leptos::*;

#[component]
pub fn PipelineCron(#[prop(into)] name: Signal<String>) -> impl IntoView {
    let data = create_resource(
        move || name.get(),
        |name| async move {
            let params = JobFiltersParams {
                pipeline: Some(name),
                ..Default::default()
            };
            api::cron(params).await.map_err(|e| e.to_string())
        },
    );
    let new_href = Signal::derive(move || format!("/cron/insert?name={}", name.get()));

    view! {
        <Card class="mt-[18px] overflow-hidden">
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
                            body="Attach a cron expression to this pipeline to have the server run it unattended."
                        >
                            <LinkButton
                                href=new_href
                                variant=ButtonVariant::Secondary
                                size=ButtonSize::Sm
                            >
                                "New schedule"
                            </LinkButton>
                        </EmptyState>
                    }
                        .into_view()
                }
                Some(Ok(jobs)) => {
                    view! {
                        {jobs
                            .into_iter()
                            .map(|job| {
                                let next = if job.is_enabled {
                                    format::cron_next(&job.schedule)
                                        .map(|d| format!("next run {}", format::until(d)))
                                        .unwrap_or_default()
                                } else {
                                    "paused".to_string()
                                };
                                view! {
                                    <div class="px-5 py-4 border-b border-line-faint flex items-center gap-3.5 flex-wrap">
                                        <span class="font-mono text-[13px] bg-chip border border-line-strong rounded-[7px] px-[9px] py-[5px]">
                                            {job.schedule.clone()}
                                        </span>
                                        <span class="text-[13px] text-ink-muted">
                                            {format!(
                                                "{} · {next}",
                                                format::cron_cadence(&job.schedule),
                                            )}
                                        </span>
                                        <LinkButton
                                            href=format!("/cron/update?id={}", job.id)
                                            variant=ButtonVariant::Ghost
                                            size=ButtonSize::Sm
                                            class="ml-auto"
                                        >
                                            "Edit"
                                        </LinkButton>
                                    </div>
                                }
                            })
                            .collect_view()}
                        <div class="px-5 py-3 flex gap-2 justify-end">
                            <LinkButton
                                href=new_href
                                variant=ButtonVariant::Secondary
                                size=ButtonSize::Sm
                            >
                                "New schedule"
                            </LinkButton>
                            <LinkButton
                                href="/cron"
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Sm
                            >
                                "Manage schedules"
                            </LinkButton>
                        </div>
                    }
                        .into_view()
                }
            }}
        </Card>
    }
}
