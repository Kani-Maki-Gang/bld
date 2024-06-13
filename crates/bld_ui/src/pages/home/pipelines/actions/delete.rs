use crate::{
    components::{
        button::{Button, IconButton},
        card::Card,
        colors::Colors,
    },
    context::{AppDialog, AppDialogContent, RefreshPipelines},
    error::SmallError,
};
use anyhow::{bail, Result};
use bld_models::dtos::PipelineQueryParams;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

type DeleteActionArgs = (
    String,
    RwSignal<Option<String>>,
    Option<RefreshPipelines>,
    bool,
    NodeRef<Dialog>,
);

async fn delete(name: String) -> Result<()> {
    let params = PipelineQueryParams { pipeline: name };

    let res = Client::builder()
        .build()?
        .delete("http://localhost:6080/v1/remove")
        .query(&params)
        .send()
        .await?;

    let status = res.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = res.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_log(&error);
        bail!(error)
    }
}

#[component]
fn PipelineDeleteButtonDialog(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshPipelines>,
    #[prop(into, optional)] redirect: bool,
) -> impl IntoView {
    let error = create_rw_signal(None);
    let delete_action = create_action(|args: &DeleteActionArgs| {
        let (id, error, refresh, redirect, dialog) = args.clone();
        async move {
            match delete(id).await {
                Ok(_) if redirect => {
                    let nav = use_navigate();
                    nav("/pipelines", NavigateOptions::default());
                    let _ = dialog.get().map(|x| x.close());
                }
                Ok(_) => {
                    let _ = refresh.map(|x| x.set());
                    let _ = dialog.get().map(|x| x.close());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        }
    });

    view! {
        <Card class="px-8 py-12 gap-4 w-[500px] h-[300px]">
            <div class="grow">
                "Are you sure you want to delete this pipeline?" <p>{move || name.get()}</p>
            </div>
            <Show when=move || error.get().is_some() fallback=|| view! {}>
                <SmallError error=move || error.get().unwrap()/>
            </Show>
            <div class="flex gap-x-4">
                <Button on:click=move |_| {
                    delete_action.dispatch((name.get(), error, refresh, redirect, app_dialog));
                }>"Delete"</Button>
                <Button on:click=move |_| {
                    let _ = app_dialog.get().map(|x| x.close());
                }>"Cancel"</Button>
            </div>
        </Card>
    }
}

#[component]
pub fn PipelineDeleteButton(
    #[prop(into)] name: Signal<String>,
    #[prop(into, optional)] redirect: bool,
) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let refresh = use_context::<RefreshPipelines>();
    view! {
        <IconButton
            icon="iconoir-bin-full"
            color=Colors::Red
            on:click=move |_| {
                let Some(AppDialog(dialog)) = app_dialog else {
                    logging::console_error("App dialog context not found");
                    return;
                };
                let Some(AppDialogContent(content)) = app_dialog_content else {
                    logging::console_error("App dialog context not found");
                    return;
                };
                let _ = content
                    .set(
                        Some(
                            view! {
                                <PipelineDeleteButtonDialog
                                    name=name
                                    app_dialog=dialog
                                    refresh=refresh
                                    redirect=redirect
                                />
                            },
                        ),
                    );
                let _ = dialog.get().map(|x| x.show_modal());
            }
        />
    }
}
