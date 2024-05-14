mod details;
mod schedule;

use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{CronJobResponse, JobFiltersParams};
use details::CronJobsEditDetails;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use schedule::CronJobsEditSchedule;
use reqwest::Client;

async fn get_cron(id: Option<String>) -> Result<CronJobResponse> {
    let id = id.ok_or_else(|| anyhow!("No id provided"))?;
    let params = JobFiltersParams {
        id: Some(id),
        ..Default::default()
    };

    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/cron")
        .query(&params)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        let data: Vec<CronJobResponse> = serde_json::from_str(&body)?;
        data.into_iter().next().ok_or_else(|| anyhow!("No data found"))
    } else {
        bail!("Request failed with status: {}", res.status())
    }
}

#[component]
pub fn CronJobsEdit() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let (cron, set_cron) = create_signal(None);
    let schedule = create_rw_signal(String::new());

    let _ = create_resource(
        move || (id(), set_cron),
        |(id, set_cron)| async move {
            let cron = get_cron(id)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .ok();
            set_cron.set(cron);
        },
    );

    create_effect(move |_| {
        if let Some(cron) = cron.get() {
            schedule.set(cron.schedule);
        }
    });

    view! {
        <Show
            when=move || cron.get().is_some()
            fallback=|| view! {
                <div class="text-2xl">
                    "Loading..."
                </div>
            }>
            <div class="flex flex-col gap-4">
                <CronJobsEditDetails job=move || cron.get().unwrap() />
                <CronJobsEditSchedule schedule=schedule />
            </div>
        </Show>
    }
}
