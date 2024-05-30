use super::{
    edit::{CronJobsEdit, SaveCronJob},
    helpers::{get_cron, get_pipeline, hash_map_strings},
};
use anyhow::{bail, Result};
use bld_models::dtos::UpdateJobRequest;
use leptos::{leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

async fn update(data: UpdateJobRequest) -> Result<()> {
    let resp = Client::builder()
        .build()?
        .patch("http://localhost:6080/v1/cron")
        .json(&data)
        .send()
        .await?;

    if resp.status().is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let msg = format!("Request failed with status: {:?}", resp);
        logging::console_error(&msg);
        bail!(msg)
    }
}

async fn delete(id: String) -> Result<()> {
    let resp = Client::builder()
        .build()?
        .delete(format!("http://localhost:6080/v1/cron/{}", id))
        .send()
        .await?;

    if resp.status().is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let msg = format!("Request failed with status: {:?}", resp);
        logging::console_error(&msg);
        bail!(msg)
    }
}

#[component]
pub fn CronJobUpdate() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let (cron, set_cron) = create_signal(None);
    let (pipeline, set_pipeline) = create_signal(None);
    let (save, set_save) = create_signal(None);
    let (get_delete, set_delete) = create_signal(());

    create_resource(
        move || (id(), set_pipeline, set_cron),
        |(id, set_pipeline, set_cron)| async move {
            let Some(id) = id else {
                return;
            };

            let cron_resp = get_cron(id.to_string())
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .ok();

            if let Some(name) = cron_resp.as_ref().map(|x| x.pipeline.clone()) {
                let pipeline_resp = get_pipeline(name)
                    .await
                    .map_err(|e| logging::console_error(e.to_string().as_str()))
                    .ok();

                if let Some(pipeline_resp) = pipeline_resp {
                    set_pipeline.set(pipeline_resp);
                }
            }

            set_cron.set(cron_resp);
        },
    );

    let save_action = create_action(|args: &(Option<String>, SaveCronJob)| {
        let (id, (schedule, vars, env)) = args;
        let id = id.clone();
        let schedule = schedule.clone();
        let vars = hash_map_strings(vars.clone());
        let env = hash_map_strings(env.clone());

        async move {
            let Some(id) = id else {
                return;
            };
            let data = UpdateJobRequest::new(id.to_string(), schedule, Some(vars), Some(env));
            let _ = update(data).await;
        }
    });

    let delete_action = create_action(|args: &String| {
        let id = args.clone();
        async move {
            let _ = delete(id).await;
        }
    });

    create_effect(move |_| {
        if let Some(data) = save.get() {
            save_action.dispatch((id(), data));
        }
    });

    let _ = watch(
        move || (get_delete.get(), id()),
        move |args: &((), Option<String>), _, _| {
            let ((), id) = args;
            if let Some(id) = id.as_ref().cloned() {
                delete_action.dispatch(id);
            }
        },
        false,
    );

    view! {
        <CronJobsEdit
            cron=cron
            pipeline=pipeline
            save=set_save
            delete=set_delete />
    }
}
