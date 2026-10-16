use crate::{
    api,
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        input::SearchInput,
        page::LoadingState,
    },
    context::use_app_dialog,
    error::{DialogCard, ErrorBanner},
};
use leptos::*;
use leptos_router::*;

#[component]
fn CronJobsNewDialog() -> impl IntoView {
    let dialog = use_app_dialog();
    let search = create_rw_signal(String::new());
    let data = create_resource(
        || (),
        |_| async move { api::list().await.map_err(|e| e.to_string()) },
    );

    view! {
        <DialogCard title="New schedule" subtitle="Pick the pipeline that the schedule runs.">
            <SearchInput placeholder="Pipeline name" value=search class="mt-4" />
            <div class="mt-3 max-h-[360px] overflow-y-auto flex flex-col">
                {move || match data.get() {
                    None => view! { <LoadingState /> }.into_view(),
                    Some(Err(e)) => view! { <ErrorBanner error=e /> }.into_view(),
                    Some(Ok(list)) => {
                        let search = search.get();
                        list.into_iter()
                            .filter(|p| search.is_empty() || p.pipeline.contains(&search))
                            .map(|p| {
                                let name = p.pipeline.clone();
                                view! {
                                    <button
                                        class="text-left px-3 py-2.5 rounded-lg hover:bg-hover font-mono text-[12.5px] text-ink-soft hover:text-ink truncate"
                                        on:click=move |_| {
                                            dialog.close();
                                            let nav = use_navigate();
                                            nav(
                                                &format!("/cron/insert?name={name}"),
                                                NavigateOptions::default(),
                                            );
                                        }
                                    >
                                        {p.pipeline}
                                    </button>
                                }
                            })
                            .collect_view()
                    }
                }}
            </div>
            <div class="mt-4 flex">
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Lg
                    class="flex-1"
                    on:click=move |_| dialog.close()
                >
                    "Close"
                </Button>
            </div>
        </DialogCard>
    }
}

#[component]
pub fn CronJobsNewButton() -> impl IntoView {
    let dialog = use_app_dialog();
    view! {
        <Button on:click=move |_| {
            dialog.open(|| view! { <CronJobsNewDialog /> })
        }>"New schedule"</Button>
    }
}
