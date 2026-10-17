mod attention;
mod health;
mod stats;

use crate::{
    api,
    components::{button::ButtonVariant, link::LinkButton, page::PageHeader},
};
use attention::DashboardAttention;
use health::DashboardHealth;
use leptos::*;
use leptos_use::use_interval_fn;
use stats::DashboardStats;

#[component]
pub fn Dashboard() -> impl IntoView {
    let health = create_resource(
        || (),
        |_| async move { api::pipeline_health().await.map_err(|e| e.to_string()) },
    );
    let refresh = create_rw_signal(());
    let _ = use_interval_fn(
        move || {
            health.refetch();
            refresh.set(());
        },
        15_000,
    );

    let subtitle = Signal::derive(move || match health.get() {
        Some(Ok(h)) if h.len() == 1 => "1 pipeline · last 30 runs each".to_string(),
        Some(Ok(h)) => format!("{} pipelines · last 30 runs each", h.len()),
        _ => String::new(),
    });

    view! {
        <PageHeader title="Overview" subtitle=subtitle>
            <LinkButton href="/files" variant=ButtonVariant::Primary>
                "Run pipeline"
            </LinkButton>
        </PageHeader>
        <DashboardAttention health=health />
        <DashboardStats refresh=refresh />
        <DashboardHealth health=health />
    }
}
