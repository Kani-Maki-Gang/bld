use leptos::*;

#[component]
pub fn PipelineRawFileV2(#[prop(into)] raw_file: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-col border border-slate-600 rounded-lg divide-y divide-slate-600">
            <div class="flex flex-col p-4">
                <div class="text-xl">"Raw file"</div>
                <div class="text-gray-400">"The raw file content of this pipeline."</div>
            </div>
            <pre class="text-sm text-gray-200 p-4">{move || raw_file.get()}</pre>
        </div>
    }
}
