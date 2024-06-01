use crate::components::card::Card;
use crate::components::list::{ComplexListItem, List};
use leptos::*;

#[component]
pub fn DashboardMostRunsPerUser() -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Most runs"</div>
                <div class="text-gray-400 mb-8">"Users with most runs in the last month"</div>
                <div class="h-96 overflow-y-auto">
                    <List>
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "John Johnson".to_string()
                            sub_title=move || "john@someemail.com".to_string()
                            stat=move || "65 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Wade Willson".to_string()
                            sub_title=move || "wade@someemail.com".to_string()
                            stat=move || "60 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Peter Parker".to_string()
                            sub_title=move || "peter@someemail.com".to_string()
                            stat=move || "49 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Charles Xavier".to_string()
                            sub_title=move || "charles@someemail.com".to_string()
                            stat= move || "40 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Bruce Wayne".to_string()
                            sub_title=move || "bruse@someemail.com".to_string()
                            stat= move || "35 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Clark Kent".to_string()
                            sub_title=move || "clark@someemail.com".to_string()
                            stat=move || "28 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Lois Lane".to_string()
                            sub_title=move || "lois@someemail.com".to_string()
                            stat=move || "24 runs".to_string() />
                        <ComplexListItem
                            icon=move || "iconoir-user-circle".to_string()
                            title=move || "Barbara Gordon".to_string()
                            sub_title=move || "barbara@someemail.com".to_string()
                            stat=move || "16 runs".to_string() />
                    </List>
                </div>
            </div>
        </Card>
    }
}
