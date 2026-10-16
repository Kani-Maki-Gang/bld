use crate::{
    api,
    components::{
        badge::RunState,
        card::Card,
        page::{EmptyState, LoadingState},
    },
    error::ErrorBanner,
    format,
};
use bld_models::dtos::HistQueryParams;
use leptos::*;
use leptos_router::A;

#[component]
pub fn PipelineHist(#[prop(into)] name: Signal<String>) -> impl IntoView {
    let data = create_resource(
        move || name.get(),
        |name| async move {
            let params = HistQueryParams {
                name: Some(name),
                limit: 50,
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
                Some(Ok(rows)) if rows.is_empty() => {
                    view! {
                        <EmptyState
                            title="No runs yet"
                            body="Runs of this pipeline show here, newest first."
                        />
                    }
                        .into_view()
                }
                Some(Ok(rows)) => {
                    rows.into_iter()
                        .map(|r| {
                            let state = RunState::parse(&r.state);
                            view! {
                                <A
                                    href=format!("/monit?id={}", r.id)
                                    class="flex items-center gap-4 px-5 py-3 border-b border-line-faint hover:bg-hover flex-wrap"
                                >
                                    <span class=format!(
                                        "size-1.5 rounded-full shrink-0 {}",
                                        state.dot_class(),
                                    )></span>
                                    <span class="font-mono text-xs text-ink-soft flex-1 min-w-0 truncate">
                                        {r.id.clone()}
                                    </span>
                                    <span class="text-xs text-ink-muted shrink-0">
                                        {format::trigger_label(&r.user)}
                                    </span>
                                    <span class="font-mono text-xs text-ink-muted shrink-0">
                                        {r
                                            .start_date_time
                                            .clone()
                                            .unwrap_or_else(|| "—".to_string())}
                                    </span>
                                    <span class="font-mono text-xs text-ink-muted w-[70px] text-right shrink-0">
                                        {format::run_duration(
                                            r.start_date_time.as_deref(),
                                            r.end_date_time.as_deref(),
                                        )}
                                    </span>
                                </A>
                            }
                        })
                        .collect_view()
                }
            }}
        </Card>
    }
}
