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
pub fn DashboardMostRunsPerUser() -> impl IntoView {
    let data = create_resource(
        || (),
        |_| async move { api::most_runs_per_user().await.map_err(|e| e.to_string()) },
    );
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Most runs"</div>
                <div class="text-gray-400 mb-8">"Users with most runs in the last month"</div>
                <div class="h-96 overflow-y-auto">
                    <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
                        <Error error=move || data.get().unwrap().unwrap_err()/>
                    </Show>
                    <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
                        <List>
                            <For
                                each=move || data.get().unwrap().unwrap()
                                key=move |i| i.user.clone()
                                let:child
                            >
                                <ComplexListItem
                                    icon=move || "iconoir-user-circle".to_string()
                                    title=move || {
                                        if child.user.is_empty() {
                                            "No user".to_string()
                                        } else {
                                            child.user.clone()
                                        }
                                    }

                                    sub_title=|| String::new()
                                    stat=move || format!("{} runs", child.count)
                                />
                            </For>
                        </List>
                    </Show>
                </div>
            </div>
        </Card>
    }
}
