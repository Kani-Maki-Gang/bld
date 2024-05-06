use crate::components::{
    badge::Badge,
    button_group::{ButtonGroup, ButtonGroupItem},
    card::Card,
    link::LinkButton,
};
use bld_runner::pipeline::v2::Pipeline;
use leptos::*;

#[component]
pub fn PipelineDetailsV2(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] name: Signal<Option<String>>,
    #[prop(into)] pipeline: Signal<Pipeline>,
    #[prop(into)] selected_group_item: RwSignal<String>,
) -> impl IntoView {
    let group = Signal::from(|| {
        vec![
            ButtonGroupItem {
                id: "view".to_string(),
                label: "View".to_string(),
            },
            ButtonGroupItem {
                id: "rawfile".to_string(),
                label: "Raw file".to_string(),
            },
        ]
    });

    let pipeline_name = move || pipeline.get().name;
    let cron = move || pipeline.get().cron.map(|x| format!("Cron: {}", x));
    let runs_on = move || format!("Runs on: {}", pipeline.get().runs_on);
    let dispose = move || format!("Dispose: {}", pipeline.get().dispose);

    view! {
        <Card>
            <div class="flex items-start px-8 py-12">
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
                        <Badge>"Version: 2"</Badge>
                        <Badge>{runs_on()}</Badge>
                        <Show when=move || cron().is_some() fallback=|| view! { }>
                            <Badge>{cron().unwrap()}</Badge>
                        </Show>
                        <Badge>{dispose()}</Badge>
                    </div>
                </div>
                <div class="flex items-center gap-x-4">
                    <div class="flex-shrink">
                        <ButtonGroup items=group selected=selected_group_item />
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
