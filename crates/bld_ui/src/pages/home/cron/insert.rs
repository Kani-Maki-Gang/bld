use super::{
    edit::{CronJobsEdit, SaveCronJob},
    helpers::{get_pipeline, hash_map_strings},
};
use crate::{
    api, context::{AppDialog, AppDialogContent}, error::{ErrorCard, ErrorDialog}
};
use anyhow::{bail, Result};
use bld_models::dtos::{AddJobRequest, CronJobResponse};
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;

type SaveActionArgs = (
    Option<String>,
    NodeRef<Dialog>,
    RwSignal<Option<View>>,
    SaveCronJob,
);

async fn insert(data: AddJobRequest) -> Result<()> {
    let res = api::cron_insert(data).await?;
    let status = res.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = res.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_error(&error);
        bail!(error)
    }
}

#[component]
pub fn CronJobInsert() -> impl IntoView {
    let params = use_query_map();
    let name = move || params.with(|p| p.get("name").cloned());
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let (save, set_save) = create_signal(None);

    let cron = move || {
        if let Some(name) = name() {
            let job = CronJobResponse {
                pipeline: name.clone(),
                ..Default::default()
            };
            Some(job)
        } else {
            None
        }
    };

    let data = create_resource(
        move || name(),
        |name| async move { get_pipeline(name).await.map_err(|e| e.to_string()) },
    );

    let save_action = create_action(|args: &SaveActionArgs| {
        let (name, dialog, dialog_content, (schedule, vars, env)) = args.clone();
        let vars = hash_map_strings(vars);
        let env = hash_map_strings(env);

        async move {
            let Some(name) = name else {
                return;
            };
            let data = AddJobRequest::new(schedule, name.to_string(), Some(vars), Some(env), false);
            let res = insert(data).await;

            if let Err(e) = res {
                dialog_content.set(Some(
                    view! { <ErrorDialog dialog=dialog error=move || e.to_string()/> },
                ));
                let _ = dialog.get().map(|x| x.show_modal());
            } else {
                let nav = use_navigate();
                nav("/cron?={}", NavigateOptions::default());
            }
        }
    });

    create_effect(move |_| {
        if let Some(data) = save.get() {
            let Some(AppDialog(dialog)) = app_dialog else {
                return;
            };
            let Some(AppDialogContent(dialog_content)) = app_dialog_content else {
                return;
            };
            save_action.dispatch((name(), dialog, dialog_content, data));
        }
    });

    view! {
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || data.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
            <CronJobsEdit
                cron=move || cron()
                pipeline=move || data.get().unwrap().ok()
                save=set_save
            />
        </Show>
    }
}
