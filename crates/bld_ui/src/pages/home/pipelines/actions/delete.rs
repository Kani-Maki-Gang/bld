use crate::{
    components::{
        button::{Button, IconButton},
        card::Card,
        colors::Colors,
    },
    context::{AppDialog, AppDialogContent, RefreshPipelines},
};
use anyhow::{bail, Result};
use bld_models::dtos::PipelineQueryParams;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

type DeleteActionArgs = (String, Option<RefreshPipelines>, bool);

async fn delete(name: String) -> Result<()> {
    let params = PipelineQueryParams { pipeline: name };

    let res = Client::builder()
        .build()?
        .delete("http://localhost:6080/v1/remove")
        .query(&params)
        .send()
        .await?;

    if res.status().is_success() {
        Ok(())
    } else {
        bail!("Request failed with status: {}", res.status())
    }
}

#[component]
fn PipelineDeleteButtonDialog(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshPipelines>,
    #[prop(into, optional)] redirect: bool
) -> impl IntoView {
    let delete_action = create_action(|args: &DeleteActionArgs| {
        let (id, refresh, redirect) = args.clone();
        async move {
            let _ = delete(id)
                .await
                .map_err(|e| logging::console_error(&e.to_string()));
            if redirect {
                let nav = use_navigate();
                nav("/pipelines", NavigateOptions::default());
            } else {
                let _ = refresh.map(|x| x.set());
            }
        }
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 w-[500px] h-[300px]">
                <div class="grow">
                    "Are you sure you want to delete this pipeline?" <p>{move || name.get()}</p>
                </div>
                <div class="flex gap-x-4">
                    <Button on:click=move |_| {
                        delete_action.dispatch((name.get(), refresh, redirect));
                        let _ = app_dialog.get().map(|x| x.close());
                    }>"Delete"</Button>
                    <Button on:click=move |_| {
                        let _ = app_dialog.get().map(|x| x.close());
                    }>"Cancel"</Button>
                </div>
            </div>
        </Card>
    }
}

#[component]
pub fn PipelineDeleteButton(
    #[prop(into)] name: Signal<String>,
    #[prop(into, optional)] redirect: bool
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
