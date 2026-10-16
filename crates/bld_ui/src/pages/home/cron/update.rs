use super::{
    edit::{CronJobsEdit, SaveCronJob},
    helpers::{get_cron, get_pipeline},
};
use crate::{api, components::page::LoadingState, context::use_app_dialog, error::ErrorCard};
use bld_models::dtos::{CronJobResponse, UpdateJobRequest};
use bld_runner::VersionedFile;
use leptos::*;
use leptos_router::*;

#[component]
pub fn CronJobUpdate() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let dialog = use_app_dialog();

    let data = create_resource(id, |id| async move {
        let cron = get_cron(id).await.map_err(|e| e.to_string())?;
        let pipeline = get_pipeline(Some(cron.pipeline.clone()))
            .await
            .map_err(|e| e.to_string())?;
        Ok::<(CronJobResponse, VersionedFile), String>((cron, pipeline))
    });

    let save = create_action(
        move |(id, (schedule, inputs, env)): &(String, SaveCronJob)| {
            let data = UpdateJobRequest::new(
                id.clone(),
                schedule.clone(),
                Some(inputs.clone()),
                Some(env.clone()),
            );
            async move {
                match api::cron_update(data).await {
                    Ok(_) => {
                        let nav = use_navigate();
                        nav("/cron", NavigateOptions::default());
                    }
                    Err(e) => dialog.error(e.to_string()),
                }
            }
        },
    );

    let delete = create_action(move |id: &String| {
        let id = id.clone();
        async move {
            match api::cron_delete(id).await {
                Ok(_) => {
                    let nav = use_navigate();
                    nav("/cron", NavigateOptions::default());
                }
                Err(e) => dialog.error(e.to_string()),
            }
        }
    });

    move || match data.get() {
        None => view! { <LoadingState /> }.into_view(),
        Some(Err(e)) => view! { <ErrorCard error=e /> }.into_view(),
        Some(Ok((cron, pipeline))) => {
            let save_id = cron.id.clone();
            let delete_id = cron.id.clone();
            view! {
                <CronJobsEdit
                    cron=cron
                    pipeline=pipeline
                    title="Edit schedule"
                    on_save=move |data| save.dispatch((save_id.clone(), data))
                    on_delete=Callback::new(move |_| delete.dispatch(delete_id.clone()))
                />
            }
            .into_view()
        }
    }
}
