use crate::components::card::Card;
use crate::components::list::{ComplexListItem, List};
use leptos::*;

#[component]
pub fn DashboardPipelines() -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Pipelines success/failure rate"</div>
                <div class="text-gray-400 mb-8">"Pipeline runs for the last month with a success/failure rate"</div>
                <div class="max-h-[600px] overflow-y-auto">
                    <List>
                        <ComplexListItem
                            icon=move || "iconoir-tools".to_string()
                            title=move || "web-app/ci.yaml".to_string()
                            stat=move || "success 80% | failure 20%".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-tools".to_string()
                            title=move || "web-app/pr.yaml".to_string()
                            stat=move || "success 80% | failure 20%".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-tools".to_string()
                            title=move || "web-app/vault-task.yaml".to_string()
                            stat=move || "success 100% | failure 0%".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-tools".to_string()
                            title=move || "web-app/refresh.yaml".to_string()
                            stat=move || "success 100% | failure 0%".to_string() />
                    </List>
                </div>
            </div>
        </Card>
    }
}
