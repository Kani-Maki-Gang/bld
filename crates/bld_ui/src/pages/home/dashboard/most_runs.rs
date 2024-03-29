use crate::components::card::Card;
use crate::components::list::{List, ListItem};
use leptos::*;

#[component]
pub fn DashboardMostRunsPerUser() -> impl IntoView {
    let (data, _set_data) = create_signal(vec![
        ListItem {
            id: "0".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "John Johnson".to_string(),
            sub_title: "john@someemail.com".to_string(),
            stat: "65 runs".to_string(),
        },
        ListItem {
            id: "1".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Wade Willson".to_string(),
            sub_title: "wade@someemail.com".to_string(),
            stat: "60 runs".to_string(),
        },
        ListItem {
            id: "2".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Peter Parker".to_string(),
            sub_title: "peter@someemail.com".to_string(),
            stat: "49 runs".to_string(),
        },
        ListItem {
            id: "3".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Charles Xavier".to_string(),
            sub_title: "charles@someemail.com".to_string(),
            stat: "40 runs".to_string(),
        },
        ListItem {
            id: "4".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Bruce Wayne".to_string(),
            sub_title: "bruse@someemail.com".to_string(),
            stat: "35 runs".to_string(),
        },
        ListItem {
            id: "5".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Clark Kent".to_string(),
            sub_title: "clark@someemail.com".to_string(),
            stat: "28 runs".to_string(),
        },
        ListItem {
            id: "6".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Lois Lane".to_string(),
            sub_title: "lois@someemail.com".to_string(),
            stat: "24 runs".to_string(),
        },
        ListItem {
            id: "7".to_string(),
            icon: "iconoir-user-circle".to_string(),
            title: "Barbara Gordon".to_string(),
            sub_title: "barbara@someemail.com".to_string(),
            stat: "16 runs".to_string(),
        },
    ]);

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Most runs"</div>
                <div class="text-gray-400 mb-8">"Users with most runs in the last month"</div>
                <div class="h-96 overflow-y-auto">
                    <List items=data />
                </div>
            </div>
        </Card>
    }
}
