use crate::{
    api,
    components::{
        card::Card,
        list::{ComplexListItem, List},
    },
    error::Error,
};
use leptos::*;

#[component]
pub fn DashboardPipelines() -> impl IntoView {
    let data = create_resource(
        || (),
        |_| async move {
            api::pipelines_per_completed_state()
                .await
                .map_err(|e| e.to_string())
        },
    );
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Pipelines success/failure rate"</div>
                <div class="text-gray-400 mb-8">
                    "Pipeline runs for the last month with a success/failure rate"
                </div>
                <div class="max-h-[600px] overflow-y-auto">
                    <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
                        <Error error=move || data.get().unwrap().unwrap_err()/>
                    </Show>
                    <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
                        <List>
                            <For
                                each=move || data.get().unwrap().unwrap()
                                key=move |e| e.pipeline.clone()
                                let:child
                            >
                                <ComplexListItem
                                    icon=move || "iconoir-tools".to_string()
                                    title=move || child.pipeline.clone()
                                    stat=move || {
                                        format!(
                                            "success {:.2}% | failure {:.2}%",
                                            child.finished_percentage,
                                            child.faulted_percentage,
                                        )
                                    }
                                />
                            </For>
                        </List>
                    </Show>
                </div>
            </div>
        </Card>
    }
}
