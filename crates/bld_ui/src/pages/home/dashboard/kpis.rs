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

    let (completed, _) = create_signal(Info {
        icon: "iconoir-check-circle".to_string(),
        count: 120,
        title: "Completed pipelines".to_string(),
        footnote: "~ 80% of pipelines complete successfully in the last 10 days".to_string(),
    });

    let (faulted, _) = create_signal(Info {
        icon: "iconoir-ev-plug-xmark".to_string(),
        count: 25,
        title: "Faulted pipelines".to_string(),
        footnote: "~ 20% of pipelines faulted in the last 10 days".to_string(),
    });

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
                    footnote: format!("{}% queued pipelines in the last 10 days", data.percentage),
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
                    count: data.count,
                    title: "Running pipelines".to_string(),
                    footnote: if data.percentage == 0.0 {
                        format!("Full worker capacity has been reached")
                    } else {
                        format!("{} worker available in the server", data.percentage)
                    },
                }
            }/>
        </Show>

        <KpiInfo info=completed/>
        <KpiInfo info=faulted/>
    }
}
