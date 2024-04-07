use leptos::*;
use leptos_router::*;

#[component]
pub fn PipelineInfo() -> impl IntoView {
    let params = use_params_map();

    let pipeline_id = move || {
        params.with(|p| p.get("id").cloned())
    };

    view! {
        "Hello from the pipeline info page for pipeline with id: " {pipeline_id}
    }
}
