use super::{edit::CronJobsEdit, helpers::get_pipeline};
use crate::{api, components::page::LoadingState, context::use_app_dialog, error::ErrorCard};
use bld_models::dtos::{AddJobRequest, CronJobResponse};
use leptos::*;
use leptos_router::*;

#[component]
pub fn CronJobInsert() -> impl IntoView {
    let params = use_query_map();
    let name = move || params.with(|p| p.get("name").cloned());
    let dialog = use_app_dialog();

    let data = create_resource(name, |name| async move {
        get_pipeline(name).await.map_err(|e| e.to_string())
    });

    let save = create_action(
        move |(name, (schedule, inputs, env)): &(String, super::edit::SaveCronJob)| {
            let data = AddJobRequest::new(
                schedule.clone(),
                name.clone(),
                Some(inputs.clone()),
                Some(env.clone()),
                false,
            );
            async move {
                match api::cron_insert(data).await {
                    Ok(_) => {
                        let nav = use_navigate();
                        nav("/cron", NavigateOptions::default());
                    }
                    Err(e) => dialog.error(e.to_string()),
                }
            }
        },
    );

    move || match data.get() {
        None => view! { <LoadingState /> }.into_view(),
        Some(Err(e)) => view! { <ErrorCard error=e /> }.into_view(),
        Some(Ok(pipeline)) => {
            let pipeline_name = name().unwrap_or_default();
            let cron = CronJobResponse {
                pipeline: pipeline_name.clone(),
                schedule: "0 0 * * * *".to_string(),
                is_enabled: true,
                ..Default::default()
            };
            view! {
                <CronJobsEdit
                    cron=cron
                    pipeline=pipeline
                    title="New schedule"
                    on_save=move |data| save.dispatch((pipeline_name.clone(), data))
                />
            }
            .into_view()
        }
    }
}
