use crate::pages::home::pipelines::{
    actions::{PipelineCopyButton, PipelineDeleteButton, PipelineMoveButton, PipelineRunButton},
    info::menu::{MenuItem, PipelinesMenu},
};
use leptos::*;

#[component]
pub fn PipelineDetails(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] name: Signal<Option<String>>,
    #[prop(into)] selected: RwSignal<MenuItem>,
) -> impl IntoView {
    view! {
        <div class="px-6 py-5 border-b border-zinc-800 grid grid-cols-4 items-center gap-4">
            <div class="grow">
                <div class="text-lg font-semibold text-white">{name}</div>
                <div class="text-xs text-zinc-500 mt-0.5">{id}</div>
            </div>
            <div class="col-span-2 flex justify-center">
                <PipelinesMenu selected=selected />
            </div>
            <div class="flex justify-end gap-x-4">
                <Show when=move || id.get().is_some() && name.get().is_some() fallback=|| view! {}>
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
                        <PipelineCopyButton name=move || name.get().unwrap() redirect=true />
                        <PipelineDeleteButton name=move || name.get().unwrap() redirect=true />
                    </div>
                </Show>
            </div>
        </div>
    }
}
