use leptos::*;
use bld_runner::VersionedPipeline;
use crate::components::{badge::Badge, card::Card};

#[component]
pub fn PipelineInfoV2(
    #[prop(into)] name: Signal<Option<String>>,
    #[prop(into)] pipeline: Signal<Option<VersionedPipeline>>,
) -> impl IntoView {
    let pipeline = move || {
        if let Some(VersionedPipeline::Version2(pip)) = pipeline.get() {
            Some(pip)
        } else {
            None
        }
    };

    let pipeline_name = move || pipeline().unwrap().name;
    let cron = move || pipeline().unwrap().cron;
    let runs_on = move || format!("{}", pipeline().unwrap().runs_on);

    view! {
        <Show
            when=move || pipeline().is_some()
            fallback=|| view! { "Invalid pipeline version" }>
            <div class="flex flex-col gap-8">
                <Card>
                    <div class="flex justify-items-center px-8 py-12">
                        <div class="grow flex flex-col gap-y-2">
                            <div class="text-2xl">
                                {name}
                            </div>
                            <Show when=move || pipeline_name().is_some() fallback=|| view! { }>
                                <div class="text-gray-400">
                                    {pipeline_name()}
                                </div>
                            </Show>
                            <div class="flex gap-x-2">
                                <Badge>"version 2"</Badge>
                                <Show when=move || cron().is_some() fallback=|| view! { }>
                                    <Badge>{cron().unwrap()}</Badge>
                                </Show>
                                <Badge>{runs_on()}</Badge>
                            </div>
                        </div>
                    </div>
                </Card>
            </div>
        </Show>
    }
}
