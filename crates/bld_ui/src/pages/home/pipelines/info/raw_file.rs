use leptos::*;

#[component]
pub fn PipelineRawFile(#[prop(into)] raw_file: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-col rounded-lg divide-y divide-zinc-800">
            <div class="flex flex-col p-4">
                <div class="text-lg font-semibold text-white">"Raw file"</div>
                <div class="text-xs text-zinc-500 mt-0.5">
                    "The raw file content of this pipeline."
                </div>
            </div>
            <pre class="text-sm text-gray-200 bg-zinc-900 rounded-lg p-4">
                {move || raw_file.get()}
            </pre>
        </div>
    }
}
