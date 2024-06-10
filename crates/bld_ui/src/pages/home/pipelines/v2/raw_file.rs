use crate::components::card::Card;
use leptos::*;

#[component]
pub fn PipelineRawFileV2(#[prop(into)] raw_file: Signal<Option<String>>) -> impl IntoView {
    view! {
        <Card class="min-h-full">
            <div class="px-8 py-12">
                <pre class="text-sm text-gray-200">
                    {move || raw_file.get()}
                </pre>
            </div>
        </Card>
    }
}
