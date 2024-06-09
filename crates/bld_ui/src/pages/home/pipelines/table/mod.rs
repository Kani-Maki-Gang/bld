use super::actions::{
    PipelineCopyButton, PipelineDeleteButton, PipelineEditButton, PipelineMoveButton,
    PipelineRunButton,
};
use crate::{components::list::List, context::RefreshPipelines};
use anyhow::Result;
use bld_models::dtos::ListResponse;
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

async fn get_pipelines() -> Result<Vec<ListResponse>> {
    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/list")
        .header("Accept", "application/json")
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        Ok(vec![])
    }
}

#[component]
pub fn PipelinesTable() -> impl IntoView {
    let refresh = use_context::<RefreshPipelines>();

    let data = create_resource(
        || (),
        |_| async move {
            get_pipelines()
                .await
                .map_err(|e| logging::console_log(e.to_string().as_str()))
                .unwrap_or_default()
                .into_iter()
                .map(|x| create_rw_signal(x))
                .collect::<Vec<RwSignal<ListResponse>>>()
        },
    );

    let _ = watch(
        move || refresh.map(|x| x.get()),
        move |_, _, _| data.refetch(),
        false,
    );

    view! {
        <List>
            <div class="divide-y divide-slate-600">
                <For
                    each=move || data.get().unwrap_or_default()
                    key=move |r| r.get().pipeline.clone()
                    let:child
                >
                    <div class="flex items-center gap-4 py-4">
                        <div class="rounded-full w-16 h-16 bg-slate-800 grid place-items-center text-xl">
                            <i class="iconoir-ease-curve-control-points"></i>
                        </div>
                        <div class="grow flex flex-col gap-2">
                            <div>{move || child.get().pipeline}</div>
                            <div class="flex text-sm text-gray-400">
                                "Id: " {move || child.get().id}
                            </div>
                        </div>
                        <div class="flex gap-2">
                            <PipelineEditButton
                                id=move || child.get().id
                                name=move || child.get().pipeline
                            />
                            <PipelineRunButton
                                id=move || child.get().id
                                name=move || child.get().pipeline
                            />
                            <PipelineMoveButton name=move || child.get().pipeline/>
                            <PipelineCopyButton name=move || child.get().pipeline/>
                            <PipelineDeleteButton name=move || child.get().pipeline/>
                        </div>
                    </div>
                </For>
            </div>
        </List>
    }
}
