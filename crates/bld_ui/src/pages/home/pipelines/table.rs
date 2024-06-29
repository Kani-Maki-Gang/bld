use super::actions::{
    PipelineCopyButton, PipelineDeleteButton, PipelineEditButton, PipelineMoveButton,
    PipelineRunButton,
};
use crate::{api, components::list::List, context::RefreshPipelines, error::Error};
use bld_models::dtos::ListResponse;
use leptos::*;
use leptos_use::signal_debounced;

#[component]
pub fn PipelinesTable(#[prop(into)] filter: Signal<String>) -> impl IntoView {
    let refresh = use_context::<RefreshPipelines>();
    let debounced_filter = signal_debounced(filter, 500.0);

    let data = create_resource(
        || (),
        |_| async move {
            api::list().await.map_err(|e| e.to_string()).map(|x| {
                x.into_iter()
                    .map(|x| create_rw_signal(x))
                    .collect::<Vec<RwSignal<ListResponse>>>()
            })
        },
    );

    let filtered_data = move || {
        let Some(Ok(data)) = data.get() else {
            return vec![];
        };
        if debounced_filter.get().is_empty() {
            data
        } else {
            data.into_iter()
                .filter(|x| x.get().pipeline.contains(filter.get().as_str()))
                .collect()
        }
    };

    let _ = watch(
        move || refresh.map(|x| x.get()),
        move |_, _, _| data.refetch(),
        false,
    );

    view! {
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <Error error=move || data.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=move || view! {}>
            <List>
                <div class="divide-y divide-slate-600">
                    <For
                        each=move || filtered_data()
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
                                <PipelineMoveButton
                                    id=move || child.get().id
                                    name=move || child.get().pipeline
                                />
                                <PipelineCopyButton name=move || child.get().pipeline/>
                                <PipelineDeleteButton name=move || child.get().pipeline/>
                            </div>
                        </div>
                    </For>
                </div>
            </List>
        </Show>
    }
}
