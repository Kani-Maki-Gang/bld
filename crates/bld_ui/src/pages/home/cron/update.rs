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
    let (save, set_save) = create_signal(None);
    let (get_delete, set_delete) = create_signal(());

    let data = create_resource(
        move || id(),
        |id| async move {
            let cron = get_cron(id)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .ok();

            let name = cron.as_ref().map(|x| x.pipeline.clone());
            let pipeline = get_pipeline(name)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .ok();

            (cron, pipeline)
        },
    );

    let cron = move || match data.get() {
        Some((cron, _)) => cron,
        None => None,
    };

    let pipeline = move || match data.get() {
        Some((_, pipeline)) => pipeline,
        None => None,
    };

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
