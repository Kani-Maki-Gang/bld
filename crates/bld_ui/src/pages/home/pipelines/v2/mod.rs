mod artifacts;
mod details;
mod external;
mod hist_and_cron;
mod jobs;
mod variables;

use crate::{
    components::card::Card,
    context::{PipelineSelectedView, PipelineView, RefreshCronJobs, RefreshHistory},
};
use bld_runner::pipeline::versioned::VersionedPipeline;
use leptos::{leptos_dom::logging, *};

use {
    artifacts::PipelineArtifactsV2, details::PipelineDetailsV2, external::PipelineExternalV2,
    hist_and_cron::PipelineHistAndCronV2, jobs::PipelineJobsV2, variables::PipelineVariablesV2,
};

#[component]
pub fn PipelineV2(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] name: Signal<Option<String>>,
    #[prop(into)] pipeline: Signal<Option<VersionedPipeline>>,
) -> impl IntoView {
    let selected_view = PipelineSelectedView(create_rw_signal(PipelineView::UI));

    let raw = move || {
        pipeline.get().map(|x| {
            serde_yaml::to_string(&x)
                .map_err(|e| logging::console_error(&format!("{e}")))
                .unwrap_or_default()
        })
    };

    let pipeline = move || {
        if let Some(VersionedPipeline::Version2(pip)) = pipeline.get() {
            Some(pip)
        } else {
            None
        }
    };

    provide_context(RefreshHistory(create_rw_signal(())));
    provide_context(RefreshCronJobs(create_rw_signal(())));
    provide_context(selected_view);

    view! {
        <Show
            when=move || pipeline().is_some()
            fallback=|| view! { "Invalid pipeline version" }>
            <div class="flex flex-col gap-8">
                <PipelineDetailsV2
                    id=id
                    name=name
                    pipeline=move || pipeline().unwrap() />
                <Show
                    when=move || matches!(selected_view.get(), PipelineView::UI)
                    fallback=|| view! {}>
                    <div class="grid grid-cols-2 gap-8">
                        <PipelineJobsV2 jobs=move || pipeline().unwrap().jobs />
                        <PipelineExternalV2 external=move || pipeline().unwrap().external />
                    </div>
                    <div class="grid grid-cols-2 gap-8">
                        <PipelineVariablesV2
                            variables=move || pipeline().unwrap().variables
                            environment=move || pipeline().unwrap().environment />
                        <PipelineArtifactsV2 artifacts=move || pipeline().unwrap().artifacts />
                    </div>
                    <PipelineHistAndCronV2 name=move || name.get() />
                </Show>
                <Show
                    when=move || matches!(selected_view.get(), PipelineView::RawFile)
                    fallback=|| view! {}>
                    <Card>
                        <div class="px-8 py-12">
                            <pre class="text-sm text-gray-200">
                                {move || raw()}
                            </pre>
                        </div>
                    </Card>
                </Show>
            </div>
        </Show>
    }
}
