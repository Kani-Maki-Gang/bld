use super::actions::{PipelineDeleteButton, PipelineMoveButton};
use crate::{
    api,
    components::{
        badge::{StateBadge, TickStrip},
        button::{ButtonSize, ButtonVariant},
        card::Card,
        link::LinkButton,
        page::LoadingState,
    },
    context::RefreshPipelines,
    error::ErrorBanner,
    pages::home::shared::{health_last, health_rate, health_state, health_ticks, pipeline_href},
};
use bld_models::dtos::{ListResponse, PipelineHealthKpi};
use leptos::*;
use leptos_router::A;
use leptos_use::signal_debounced;
use std::collections::HashMap;

#[component]
fn PipelineCard(
    #[prop()] pipeline: ListResponse,
    #[prop()] health: Option<PipelineHealthKpi>,
) -> impl IntoView {
    let (state, ticks, summary) = match &health {
        Some(h) => (
            health_state(h),
            health_ticks(h),
            format!("{} · {}", health_last(h), health_rate(h)),
        ),
        None => (
            crate::components::badge::RunState::NoRun,
            vec![],
            "never run".to_string(),
        ),
    };
    let id = pipeline.id.clone();
    let name = pipeline.pipeline.clone();
    let info_href = pipeline_href(&pipeline.id, &pipeline.pipeline);
    let run_href = format!(
        "/files/run?id={}&name={}",
        pipeline.id, pipeline.pipeline
    );
    view! {
        <Card class="px-5 py-[18px] flex flex-col gap-3.5 hover:border-[#2f3543] transition-colors">
            <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                    <A
                        href=info_href
                        class="block font-mono text-[13.5px] font-medium truncate hover:text-accent-link"
                    >
                        {pipeline.pipeline}
                    </A>
                    <div class="font-mono text-[11px] text-ink-faint mt-1 truncate">
                        {pipeline.id}
                    </div>
                </div>
                <StateBadge state=state />
            </div>
            <TickStrip states=ticks tick_class="max-w-[9px] h-4" />
            <div class="flex items-center justify-between gap-3 pt-3 border-t border-line-soft">
                <span class="text-xs text-ink-muted truncate">{summary}</span>
                <div class="flex gap-1.5 shrink-0">
                    <LinkButton href=run_href variant=ButtonVariant::Secondary size=ButtonSize::Sm>
                        "Run"
                    </LinkButton>
                    <PipelineMoveButton
                        id=Signal::derive(move || id.clone())
                        name=Signal::derive({
                            let name = name.clone();
                            move || name.clone()
                        })
                        size=ButtonSize::Sm
                    />
                    <PipelineDeleteButton name=Signal::derive(move || name.clone()) icon=true />
                </div>
            </div>
        </Card>
    }
}

#[component]
pub fn PipelineCards(#[prop(into)] filter: Signal<String>) -> impl IntoView {
    let refresh = use_context::<RefreshPipelines>();
    let filter = signal_debounced(filter, 300.0);

    let data = create_resource(
        move || refresh.map(|r| r.get()),
        |_| async move {
            let list = api::list().await.map_err(|e| e.to_string())?;
            let health: HashMap<String, PipelineHealthKpi> = api::pipeline_health()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|h| (h.pipeline.clone(), h))
                .collect();
            Ok::<_, String>((list, health))
        },
    );

    move || {
        match data.get() {
        None => view! { <LoadingState /> }.into_view(),
        Some(Err(e)) => view! {
            <div class="mt-[18px]">
                <ErrorBanner error=e />
            </div>
        }.into_view(),
        Some(Ok((list, _))) if list.is_empty() => view! {
            <div class="mt-[18px] border border-dashed border-line-edge rounded-[14px] px-[22px] py-[52px] flex flex-col items-center gap-2.5 text-center bg-[#111318]">
                <div class="size-[30px] rounded-[9px] border border-line-edge grid place-items-center bg-hover">
                    <div class="size-[11px] rounded-[3px] border-[1.5px] border-dashed border-ink-faint rotate-45"></div>
                </div>
                <div class="text-[15px] font-semibold mt-1.5 tracking-[-.01em]">
                    "No files on this server"
                </div>
                <div class="text-[12.5px] text-ink-muted max-w-[400px] leading-[1.55]">
                    "Push a " <span class="font-mono">".yaml"</span>
                    " pipeline with the CLI. It shows here after the push."
                </div>
                <div class="mt-2 font-mono text-xs text-ink-soft border border-line-strong rounded-[9px] h-8 px-3 inline-flex items-center">
                    "bld push pipeline.yaml"
                </div>
            </div>
        }
        .into_view(),
        Some(Ok((list, mut health))) => {
            let filter = filter.get();
            let cards: Vec<_> = list
                .into_iter()
                .filter(|p| filter.is_empty() || p.pipeline.contains(&filter))
                .collect();
            if cards.is_empty() {
                return view! {
                    <div class="mt-[18px] text-center text-[12.5px] text-ink-muted py-10">
                        "No files match this search."
                    </div>
                }
                .into_view();
            }
            view! {
                <div class="mt-[18px] grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-3.5">
                    {cards
                        .into_iter()
                        .map(|p| {
                            let h = health.remove(&p.pipeline);
                            view! { <PipelineCard pipeline=p health=h /> }
                        })
                        .collect_view()}
                </div>
            }
            .into_view()
        }
    }
    }
}
