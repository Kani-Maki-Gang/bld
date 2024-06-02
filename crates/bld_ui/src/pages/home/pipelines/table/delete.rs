use crate::{
    components::{button::{Button, IconButton}, card::Card, colors::Colors},
    context::{AppDialog, AppDialogContent, RefreshPipelines}
};
use anyhow::{bail, Result};
use bld_models::dtos::PipelineQueryParams;
use leptos::{leptos_dom::logging, html::Dialog, *};
use reqwest::Client;

async fn delete(name: String) -> Result<()> {
    let params = PipelineQueryParams {
        pipeline: name
    };

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
fn PipelineTableDeleteButtonDialog(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshPipelines>,
) -> impl IntoView {
    let delete_action = create_action(|args: &(String, Option<RefreshPipelines>)| {
        let (id, refresh) = args.clone();
        async move {
            let _ = delete(id)
                .await
                .map_err(|e| logging::console_error(&e.to_string()));
            let _ = refresh.map(|x| x.set());
        }
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 w-[500px] h-[300px]">
                <div class="grow">
                    "Are you sure you want to delete this pipeline?"
                    <p>
                        {move || name.get()}
                    </p>
                </div>
                <div class="flex gap-x-4">
                    <Button on:click=move |_| {
                        delete_action.dispatch((name.get(), refresh));
                        let _ = app_dialog.get().map(|x| x.close());
                    }>
                        "Delete"
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
pub fn PipelineTableDeleteButton(#[prop(into)] name: Signal<String>) -> impl IntoView {
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

                let _ = content.set(Some(view! {
                    <PipelineTableDeleteButtonDialog name=name app_dialog=dialog refresh=refresh />
                }));

                let _ = dialog.get().map(|x| x.show());
            }/>
    }
}
