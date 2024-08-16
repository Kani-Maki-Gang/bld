use crate::{
    api,
    components::kpi::{Info, KpiInfo},
    error::ErrorCard,
};
use leptos::*;

#[component]
pub fn DashboardKpis() -> impl IntoView {
    let queued_resource = create_resource(
        || (),
        |_| async move { api::queued_pipelines().await.map_err(|e| e.to_string()) },
    );

    let running_resource = create_resource(
        || (),
        |_| async move { api::running_pipelines().await.map_err(|e| e.to_string()) },
    );

    let completed_resource = create_resource(
        || (),
        |_| async move { api::completed_pipelines().await.map_err(|e| e.to_string()) },
    );

    view! {
        <Show when=move || matches!(queued_resource.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || queued_resource.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(queued_resource.get(), Some(Ok(_))) fallback=|| view! {}>
            <KpiInfo info=move || {
                let data = queued_resource.get().unwrap().unwrap();
                Info {
                    icon: "iconoir-timer".to_string(),
                    count: data.count,
                    title: "Queued pipelines".to_string(),
                    footnote: String::new(),
                }
            }/>
        </Show>

        <Show when=move || matches!(running_resource.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || running_resource.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(running_resource.get(), Some(Ok(_))) fallback=|| view! {}>
            <KpiInfo info=move || {
                let data = running_resource.get().unwrap().unwrap();
                Info {
                    icon: "iconoir-running".to_string(),
                    count: data.count as u64,
                    title: "Running pipelines".to_string(),
                    footnote: if data.available_workers == 0 {
                        format!("Full worker capacity has been reached")
                    } else {
                        format!("{} worker available in the server", data.available_workers)
                    },
                }
            }/>
        </Show>

        <Show when=move || matches!(completed_resource.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || completed_resource.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(completed_resource.get(), Some(Ok(_))) fallback=|| view! {}>
            <KpiInfo info=move || {
                let data = completed_resource.get().unwrap().unwrap();
                Info {
                    icon: "iconoir-check-circle".to_string(),
                    count: data.finished_count as u64,
                    title: "Finished pipelines".to_string(),
                    footnote: format!(
                        "{:.2}% of pipelines complete successfully in the last 10 days",
                        data.finished_percentage,
                    ),
                }
            }/>
        </Show>

        <Show when=move || matches!(completed_resource.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || completed_resource.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(completed_resource.get(), Some(Ok(_))) fallback=|| view! {}>
            <KpiInfo info=move || {
                let data = completed_resource.get().unwrap().unwrap();
                Info {
                    icon: "iconoir-ev-plug-xmark".to_string(),
                    count: data.faulted_count as u64,
                    title: "Faulted pipelines".to_string(),
                    footnote: format!(
                        "{:.2}% of pipelines faulted in the last 10 days",
                        data.faulted_percentage,
                    ),
                }
            }/>
        </Show>
    }
}
