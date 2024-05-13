use anyhow::Result;
use bld_models::dtos::{CronJobResponse, JobFiltersParams};
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

async fn get_cron(params: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/cron")
        .query(params)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        Ok(vec![])
    }
}

#[component]
pub fn CronJobsEdit() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let (cron, set_cron) = create_signal(vec![]);

    let _ = create_resource(
        move || (id(), set_cron),
        |(id, set_cron)| async move {
            let Some(id) = id else {
                return;
            };

            let mut params = JobFiltersParams::default();
            params.id = Some(id);
            let cron = get_cron(&params)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .unwrap_or_default();
            set_cron.set(cron);
        },
    );

    view! {}
}
