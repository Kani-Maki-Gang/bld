use crate::components::kpi::{Info, KpiInfo};
use leptos::*;

#[component]
pub fn DashboardKpis() -> impl IntoView {
    let (queued, _) = create_signal(Info {
        icon: "iconoir-timer".to_string(),
        count: 10,
        title: "Queued pipelines".to_string(),
        footnote: "+ 10% more queued pipelines in the last 10 days".to_string(),
    });

    let (running, _) = create_signal(Info {
        icon: "iconoir-running".to_string(),
        count: 30,
        title: "Running pipelines".to_string(),
        footnote: "Full worker capacity has been reached multiple times in the last 10 days"
            .to_string(),
    });

    let (completed, _) = create_signal(Info {
        icon: "iconoir-check-circle".to_string(),
        count: 120,
        title: "Completed pipelines".to_string(),
        footnote: "~ 80% of pipelines complete successfully in the last 10 days".to_string(),
    });

    let (faulted, _) = create_signal(Info {
        icon: "iconoir-ev-plug-xmark".to_string(),
        count: 25,
        title: "Fauled pipelines".to_string(),
        footnote: "~ 20% of pipelines faulted in the last 10 days".to_string(),
    });

    view! {
        <KpiInfo info=queued />
        <KpiInfo info=running />
        <KpiInfo info=completed />
        <KpiInfo info=faulted />
    }
}
