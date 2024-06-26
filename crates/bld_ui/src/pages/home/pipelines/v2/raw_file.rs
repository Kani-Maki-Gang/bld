use crate::components::card::Card;
use leptos::*;

#[component]
pub fn PipelineRawFileV2(#[prop(into)] raw_file: Signal<Option<String>>) -> impl IntoView {
    view! {
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12 gap-y-4">
                <div class="flex flex-col">
                    <div class="text-xl">"Raw file"</div>
                    <div class="text-gray-400">"The raw file content of this pipeline."</div>
                </div>
                <pre class="text-sm text-gray-200">{move || raw_file.get()}</pre>
            </div>
        </Card>
    }
}
