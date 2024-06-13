use super::{
    edit::{CronJobsEdit, SaveCronJob},
    helpers::{get_cron, get_pipeline, hash_map_strings},
};
use crate::{
    components::card::Card,
    context::{AppDialog, AppDialogContent},
    error::{Error, ErrorDialog},
};
use anyhow::{bail, Result};
use bld_models::dtos::{CronJobResponse, UpdateJobRequest};
use bld_runner::VersionedPipeline;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

type UpdateActionArgs = (
    Option<String>,
    NodeRef<Dialog>,
    RwSignal<Option<View>>,
    SaveCronJob,
);

type DeleteActionArgs = (String, NodeRef<Dialog>, RwSignal<Option<View>>);

async fn update(data: UpdateJobRequest) -> Result<()> {
    let resp = Client::builder()
        .build()?
        .patch("http://localhost:6080/v1/cron")
        .json(&data)
        .send()
        .await?;

    let status = resp.status();
    if status.is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let body = resp.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_error(&error);
        bail!(error)
    }
}

async fn delete(id: String) -> Result<()> {
    let resp = Client::builder()
        .build()?
        .delete(format!("http://localhost:6080/v1/cron/{}", id))
        .send()
        .await?;

    let status = resp.status();
    if status.is_success() {
        let nav = use_navigate();
        nav("/cron?={}", NavigateOptions::default());
        Ok(())
    } else {
        let body = resp.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_error(&error);
        bail!(error)
    }
}

#[component]
pub fn CronJobUpdate() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let (save, set_save) = create_signal(None);
    let (get_delete, set_delete) = create_signal(false);
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();

    let data = create_resource(
        move || id(),
        |id| async move {
            let cron = get_cron(id).await.map_err(|e| e.to_string())?;
            let name = cron.pipeline.clone();
            let pipeline = get_pipeline(Some(name)).await.map_err(|e| e.to_string())?;
            Ok::<(CronJobResponse, VersionedPipeline), String>((cron, pipeline))
        },
    );

    let cron = move || match data.get() {
        Some(Ok((cron, _))) => Some(cron),
        _ => None,
    };

    let pipeline = move || match data.get() {
        Some(Ok((_, pipeline))) => Some(pipeline),
        _ => None,
    };

    let save_action = create_action(|args: &UpdateActionArgs| {
        let (id, dialog, content, (schedule, vars, env)) = args.clone();
        let vars = hash_map_strings(vars);
        let env = hash_map_strings(env);

        async move {
            let Some(id) = id else {
                return;
            };
            let data = UpdateJobRequest::new(id.to_string(), schedule, Some(vars), Some(env));
            let res = update(data).await;
            if let Err(e) = res {
                content.set(Some(
                    view! { <ErrorDialog dialog=dialog error=move || e.to_string()/> },
                ));
                let _ = dialog.get().map(|x| x.show_modal());
            }
        }
    });

    let delete_action = create_action(|args: &DeleteActionArgs| {
        let (id, dialog, content) = args.clone();
        async move {
            let res = delete(id).await;
            if let Err(e) = res {
                content.set(Some(
                    view! { <ErrorDialog dialog=dialog error=move || e.to_string()/> },
                ));
                let _ = dialog.get().map(|x| x.show_modal());
            }
        }
    });

    create_effect(move |_| {
        if let Some(data) = save.get() {
            let Some(AppDialog(dialog)) = app_dialog else {
                return;
            };
            let Some(AppDialogContent(content)) = app_dialog_content else {
                return;
            };
            save_action.dispatch((id(), dialog, content, data));
        }
    });

    create_effect(move |_| {
        if get_delete.get() {
            let Some(AppDialog(dialog)) = app_dialog else {
                return;
            };

            let Some(AppDialogContent(content)) = app_dialog_content else {
                return;
            };

            if let Some(id) = id().as_ref().cloned() {
                delete_action.dispatch((id, dialog, content));
            }
        }
    });

    view! {
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <div class="flex flex-col items-center">
                <Card class="container px-8 py-12">
                    <Error error=move || data.get().unwrap().unwrap_err()/>
                </Card>
            </div>
        </Show>
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
            <CronJobsEdit cron=cron pipeline=pipeline save=set_save delete=set_delete/>
        </Show>
    }
}
