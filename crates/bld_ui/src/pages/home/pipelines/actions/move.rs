use crate::{
    components::{
        button::{Button, IconButton},
        card::Card,
        colors::Colors,
        input::Input,
    },
    context::{AppDialog, AppDialogContent, RefreshPipelines},
};
use anyhow::{bail, Result};
use bld_models::dtos::PipelinePathRequest;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
use reqwest::Client;

type MoveActionArgs = (String, String, String, Option<RefreshPipelines>, bool);

async fn api_move(pipeline: String, target: String) -> Result<()> {
    let params = PipelinePathRequest { pipeline, target };

    let res = Client::builder()
        .build()?
        .patch("http://localhost:6080/v1/move")
        .json(&params)
        .send()
        .await?;

    if res.status().is_success() {
        Ok(())
    } else {
        bail!("Request failed with status: {}", res.status())
    }
}

#[component]
fn PipelineMoveButtonDialog(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshPipelines>,
    #[prop(into, optional)] redirect: bool,
) -> impl IntoView {
    let name_rw = create_rw_signal(name.get_untracked());
    let target = create_rw_signal(String::new());
    let delete_action = create_action(|args: &MoveActionArgs| {
        let (id, pipeline, target, refresh, redirect) = args.clone();
        async move {
            let move_res = api_move(pipeline, target.clone())
                .await
                .map_err(|e| logging::console_error(&e.to_string()));
            if redirect && move_res.is_ok() {
                let nav = use_navigate();
                let route = format!("/pipelines/info?id={id}&name={target}");
                logging::console_log(&format!("rerouting to {route}"));
                nav(&route, NavigateOptions::default());
            } else if move_res.is_ok() {
                let _ = refresh.map(|x| x.set());
            }
        }
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4 w-[500px] h-[400px]">
                <div>"Move(Rename) pipeline"</div>
                <div class="grow flex flex-col gap-4">
                    <div>
                        <label for="pipeline">Current:</label>
                        <Input id="pipeline" disabled=true value=name_rw/>
                    </div>
                    <div>
                        <label for="target">New:</label>
                        <Input id="target" value=target/>
                    </div>
                </div>
                <div class="flex gap-x-4">
                    <Button on:click=move |_| {
                        delete_action
                            .dispatch((id.get(), name.get(), target.get(), refresh, redirect));
                        let _ = app_dialog.get().map(|x| x.close());
                    }>"Ok"</Button>
                    <Button on:click=move |_| {
                        let _ = app_dialog.get().map(|x| x.close());
                    }>"Cancel"</Button>
                </div>
            </div>
        </Card>
    }
}

#[component]
pub fn PipelineMoveButton(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] name: Signal<String>,
    #[prop(into, optional)] redirect: bool,
) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let refresh = use_context::<RefreshPipelines>();
    view! {
        <IconButton
            icon="iconoir-scissor"
            color=Colors::Zinc
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
                                <PipelineMoveButtonDialog
                                    id=move || id.get()
                                    name=move || name.get()
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