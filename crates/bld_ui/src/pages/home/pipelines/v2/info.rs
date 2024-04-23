use crate::components::{badge::Badge, button::Button, card::Card, list::ListItem, table::TableRow};
use bld_runner::VersionedPipeline;
use leptos::*;
use std::collections::HashMap;

use super::{jobs::PipelineJobsV2, variables::PipelineVariablesV2, artifacts::PipelineArtifactsV2};

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
    let variables = move || pipeline().unwrap().variables;
    let environment = move || pipeline().unwrap().environment;

    let artifact_headers = Signal::from(|| vec![
        "Method".into_view(),
        "From".into_view(),
        "To".into_view(),
        "Ignore errors".into_view(),
        "After".into_view(),
    ]);

    let artifact_rows = move || {
        pipeline()
            .unwrap()
            .artifacts
            .into_iter()
            .map(|x| TableRow {
                columns: vec![
                    x.method.into_view(),
                    x.from.into_view(),
                    x.to.into_view(),
                    x.ignore_errors.into_view(),
                    x.after.into_view(),
                ],
            })
            .collect::<Vec<TableRow>>()
    };

    let jobs = move || {
        pipeline()
            .unwrap()
            .jobs
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    v.into_iter()
                        .map(|x| {
                            let mut item = ListItem::default();
                            item.content = Some(serde_yaml::to_string(&x).unwrap().into_view());
                            item
                        })
                        .collect::<Vec<ListItem>>(),
                )
            })
            .collect::<HashMap<String, Vec<ListItem>>>()
    };

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
                                <Badge>{runs_on()}</Badge>
                                <Show when=move || cron().is_some() fallback=|| view! { }>
                                    <Badge>{cron().unwrap()}</Badge>
                                </Show>
                            </div>
                        </div>
                        <div class="flex gap-x-4">
                            <div class="min-w-40">
                                <Button>"Edit"</Button>
                            </div>
                            <div class="min-w-40">
                                <Button>"Run"</Button>
                            </div>
                        </div>
                    </div>
                </Card>
                <div class="grid grid-cols-2 gap-8">
                    <PipelineVariablesV2 variables=variables environment=environment />
                    <PipelineArtifactsV2 headers=artifact_headers rows=artifact_rows />
                </div>
                <PipelineJobsV2 jobs=jobs />
            </div>
        </Show>
    }
}
