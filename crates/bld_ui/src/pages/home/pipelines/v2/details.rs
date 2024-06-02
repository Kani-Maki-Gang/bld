use crate::{
    components::{
        badge::Badge,
        button_group::{ButtonGroup, ButtonGroupItem},
        card::Card,
        link::LinkButton,
    },
    context::{PipelineSelectedView, PipelineView},
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
    let selected_view = use_context::<PipelineSelectedView>();

    view! {
        <Card>
            <div class="flex items-start px-8 py-12">
                <div class="grow flex flex-col gap-y-2">
                    <div class="text-2xl">
                        {name}
                    </div>
                    <Show when=move || pipeline_name().is_some() fallback=|| view! { }>
                        <div class="text-gray-400">
                            {move || pipeline_name()}
                        </div>
                    </Show>
                    <div class="flex gap-x-2">
                        <Badge>"Version: 2"</Badge>
                        <Badge>{move || runs_on()}</Badge>
                        <Show when=move || cron().is_some() fallback=|| view! { }>
                            <Badge>{move || cron().unwrap()}</Badge>
                        </Show>
                        <Badge>{move || dispose()}</Badge>
                    </div>
                </div>
                <div class="flex items-center gap-x-4">
                    <div class="flex-shrink">
                        <ButtonGroup>
                            <ButtonGroupItem
                                is_selected=move || selected_view.map(|x| matches!(x.get(), PipelineView::UI)).unwrap_or_default()
                                on:click=move |_| {
                                    let _ = selected_view.map(|x| x.set(PipelineView::UI));
                                }>
                                "View"
                            </ButtonGroupItem>
                            <ButtonGroupItem
                                is_selected=move || selected_view.map(|x| matches!(x.get(), PipelineView::RawFile)).unwrap_or_default()
                                on:click=move |_| {
                                    let _ = selected_view.map(|x| x.set(PipelineView::RawFile));
                                }>
                                "Raw file"
                            </ButtonGroupItem>
                        </ButtonGroup>
                    </div>
                    <Show
                        when=move || id.get().is_some() && name.get().is_some()
                        fallback=|| view! { }>
                        <div class="w-40 flex">
                            <LinkButton
                                href=move || format!("/pipelines/run?id={}&name={}", id.get().unwrap(), name.get().unwrap())>
                                "Run"
                            </LinkButton>
                        </div>
                    </Show>
                </div>
            </div>
        </Card>
    }
}
