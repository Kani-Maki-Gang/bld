use crate::components::card::Card;
use crate::components::list::{List, ListItem};
use leptos::*;

#[component]
pub fn DashboardPipelines() -> impl IntoView {
    let (data, _set_data) = create_signal(vec![
        ListItem {
            id: "0".to_string(),
            icon: "iconoir-tools".to_string(),
            title: "web-app/ci.yaml".to_string(),
            sub_title: None,
            content: None,
            stat: "success 80% | failure 20%".to_string(),
        },
        ListItem {
            id: "1".to_string(),
            icon: "iconoir-tools".to_string(),
            title: "web-app/pr.yaml".to_string(),
            sub_title: None,
            content: None,
            stat: "success 76% | failure 24%".to_string(),
        },
        ListItem {
            id: "2".to_string(),
            icon: "iconoir-tools".to_string(),
            title: "web-app/vault-task.yaml".to_string(),
            sub_title: None,
            content: None,
            stat: "success 100% | failure 0%".to_string(),
        },
        ListItem {
            id: "3".to_string(),
            icon: "iconoir-tools".to_string(),
            title: "redis/refresh.yaml".to_string(),
            sub_title: None,
            content: None,
            stat: "success 100% | failure 0%".to_string(),
        },
    ]);

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Pipelines success/failure rate"</div>
                <div class="text-gray-400 mb-8">"Pipeline runs for the last month with a success/failure rate"</div>
                <div class="max-h-[600px] overflow-y-auto">
                    <List items=data />
                </div>
            </div>
        </Card>
    }
}
