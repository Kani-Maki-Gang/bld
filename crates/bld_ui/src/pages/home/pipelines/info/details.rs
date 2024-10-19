use crate::pages::home::pipelines::actions::{
    PipelineCopyButton, PipelineDeleteButton, PipelineMoveButton, PipelineRunButton,
};
use leptos::*;

#[component]
pub fn PipelineDetailsV2(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] name: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="flex items-start">
            <div class="grow text-2xl">{name}</div>
            <div class="flex items-center gap-x-4">
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
