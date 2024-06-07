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
use reqwest::Client;

async fn copy(pipeline: String, target: String) -> Result<()> {
    let params = PipelinePathRequest { pipeline, target };

    let res = Client::builder()
        .build()?
        .post("http://localhost:6080/v1/copy")
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
fn PipelineTableCopyButtonDialog(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshPipelines>,
) -> impl IntoView {
    let name_rw = create_rw_signal(String::new());
    let target = create_rw_signal(String::new());
    let delete_action = create_action(|args: &(String, String, Option<RefreshPipelines>)| {
        let (pipeline, target, refresh) = args.clone();
        async move {
            let _ = copy(pipeline, target)
                .await
                .map_err(|e| logging::console_error(&e.to_string()));
            let _ = refresh.map(|x| x.set());
        }
    });

    create_effect(move |_| {
        name_rw.set(name.get());
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4 w-[500px] h-[400px]">
                <div>
                    "Create a new copy"
                </div>
                <div class="grow flex flex-col gap-4">
                    <div>
                        <label for="pipeline">Pipeline:</label>
                        <Input id="pipeline" disabled=true value=name_rw/>
                    </div>
                    <div>
                        <label for="target">Copy:</label>
                        <Input id="target" value=target />
                    </div>
                </div>
                <div class="flex gap-x-4">
                    <Button on:click=move |_| {
                        delete_action.dispatch((name.get(), target.get(), refresh));
                        let _ = app_dialog.get().map(|x| x.close());
                    }>
                        "Ok"
                    </Button>
                    <Button on:click=move |_| {
                        let _ = app_dialog.get().map(|x| x.close());
                    }>
                        "Cancel"
                    </Button>
                </div>
            </div>
        </Card>
    }
}

#[component]
pub fn PipelineTableCopyButton(#[prop(into)] name: Signal<String>) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let refresh = use_context::<RefreshPipelines>();
    view! {
        <IconButton
            icon="iconoir-copy"
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

                let _ = content.set(Some(view! {
                    <PipelineTableCopyButtonDialog name=move || name.get() app_dialog=dialog refresh=refresh/>
                }));

                let _ = dialog.get().map(|x| x.show_modal());
            }/>
    }
}
