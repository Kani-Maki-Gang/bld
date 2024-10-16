use crate::{
    components::{badge::Badge, card::Card},
    pages::home::pipelines::actions::{
        PipelineCopyButton, PipelineDeleteButton, PipelineMoveButton, PipelineRunButton,
    },
};
use bld_runner::pipeline::v2::Pipeline;
use leptos::*;

#[component]
pub fn PipelineDetailsV2(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] name: Signal<Option<String>>,
    #[prop(into)] pipeline: Signal<Pipeline>,
) -> impl IntoView {
    let pipeline_name = move || pipeline.get().name;
    let cron = move || pipeline.with(|p| p.cron.as_ref().map(|x| format!("Cron: {x}")));
    let runs_on = move || pipeline.with(|p| format!("Runs on: {}", p.runs_on));
    let dispose = move || pipeline.with(|p| format!("Dispose: {}", p.dispose));
    let registry =
        move || pipeline.with(|p| p.runs_on.registry().map(|x| format!("Registry: {x}")));
    let username = move || {
        pipeline.with(|p| {
            p.runs_on
                .registry_username()
                .map(|x| format!("Username: {x}"))
        })
    };

    view! {
        <Card>
            <div class="flex items-start px-8 py-12">
                <div class="grow flex flex-col gap-y-2">
                    <div class="text-2xl">{name}</div>
                    <Show when=move || pipeline_name().is_some() fallback=|| view! {}>
                        <div class="text-gray-400">{move || pipeline_name()}</div>
                    </Show>
                    <div class="flex gap-x-2">
                        <Badge>"Version: 2"</Badge>
                        <Badge>{move || runs_on()}</Badge>
                        <Show when=move || registry().is_some() fallback=|| view! {}>
                            <Badge>{move || registry()}</Badge>
                        </Show>
                        <Show when=move || username().is_some() fallback=|| view! {}>
                            <Badge>{move || username()}</Badge>
                        </Show>
                        <Show when=move || cron().is_some() fallback=|| view! {}>
                            <Badge>{move || cron().unwrap()}</Badge>
                        </Show>
                        <Badge>{move || dispose()}</Badge>
                    </div>
                </div>
                <div class="flex items-center gap-x-4">
                    <Show
                        when=move || id.get().is_some() && name.get().is_some()
                        fallback=|| view! {}
                    >
                        <div class="flex gap-2">
                            <PipelineRunButton
                                id=move || id.get().unwrap()
                                name=move || name.get().unwrap()
                            />
                            <PipelineMoveButton
                                id=move || id.get().unwrap()
                                name=move || name.get().unwrap()
                                redirect=true
                            />
                            <PipelineCopyButton name=move || name.get().unwrap() redirect=true/>
                            <PipelineDeleteButton name=move || name.get().unwrap() redirect=true/>
                        </div>
                    </Show>
                </div>
            </div>
        </Card>
    }
}
