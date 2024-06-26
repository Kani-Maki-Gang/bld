mod artifacts;
mod cron;
mod details;
mod external;
mod hist;
mod jobs;
mod menu;
mod raw_file;
mod variables;

use crate::context::{RefreshCronJobs, RefreshHistory};
use bld_runner::pipeline::versioned::VersionedPipeline;
use leptos::{leptos_dom::logging, *};

use {
    artifacts::PipelineArtifactsV2, cron::PipelineCronV2, details::PipelineDetailsV2,
    external::PipelineExternalV2, hist::PipelineHistV2, jobs::PipelineJobsV2,
    menu::PipelinesV2Menu, raw_file::PipelineRawFileV2, variables::PipelineVariablesV2,
};

#[component]
pub fn PipelineV2(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] name: Signal<Option<String>>,
    #[prop(into)] pipeline: Signal<Option<VersionedPipeline>>,
) -> impl IntoView {
    let selected_menu_item = create_rw_signal(menu::MenuItem::Jobs);

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

    view! {
        <Show when=move || pipeline().is_some() fallback=|| view! { "Invalid pipeline version" }>
            <div class="h-full flex flex-col gap-4">
                <PipelineDetailsV2 id=id name=name pipeline=move || pipeline().unwrap()/>
                <div class="grow flex gap-4">
                    <div class="flex-none w-48">
                        <PipelinesV2Menu selected=selected_menu_item/>
                    </div>
                    <div class="grow">
                        <Show
                            when=move || matches!(selected_menu_item.get(), menu::MenuItem::Jobs)
                            fallback=|| view! {}
                        >
                            <PipelineJobsV2 jobs=move || pipeline().unwrap().jobs/>
                        </Show>
                        <Show
                            when=move || {
                                matches!(selected_menu_item.get(), menu::MenuItem::External)
                            }
                            fallback=|| view! {}
                        >
                            <PipelineExternalV2 external=move || pipeline().unwrap().external/>
                        </Show>
                        <Show
                            when=move || {
                                matches!(selected_menu_item.get(), menu::MenuItem::Variables)
                            }
                            fallback=|| view! {}
                        >
                            <PipelineVariablesV2
                                title="Variables"
                                subtitle="The configured variables for this pipeline"
                                no_data_text=move || "No variables configured".to_string()
                                variables=move || pipeline().unwrap().variables
                            />
                        </Show>
                        <Show
                            when=move || {
                                matches!(selected_menu_item.get(), menu::MenuItem::Environment)
                            }
                            fallback=|| view! {}
                        >
                            <PipelineVariablesV2
                                title="Environment variables"
                                subtitle="The configured environment variables for this pipeline"
                                no_data_text=move || {
                                    "No environment variables configured".to_string()
                                }
                                variables=move || pipeline().unwrap().environment
                            />
                        </Show>
                        <Show
                            when=move || {
                                matches!(selected_menu_item.get(), menu::MenuItem::Artifacts)
                            }
                            fallback=|| view! {}
                        >
                            <PipelineArtifactsV2 artifacts=move || pipeline().unwrap().artifacts/>
                        </Show>
                        <Show
                            when=move || matches!(selected_menu_item.get(), menu::MenuItem::History)
                            fallback=|| view! {}
                        >
                            <PipelineHistV2 name=move || name.get()/>
                        </Show>
                        <Show
                            when=move || matches!(selected_menu_item.get(), menu::MenuItem::Cron)
                            fallback=|| view! {}
                        >
                            <PipelineCronV2 name=move || name.get()/>
                        </Show>
                        <Show
                            when=move || matches!(selected_menu_item.get(), menu::MenuItem::RawFile)
                            fallback=|| view! {}
                        >
                            <PipelineRawFileV2 raw_file=move || raw()/>
                        </Show>
                    </div>
                </div>
            </div>
        </Show>
    }
}
