use crate::{
    components::{button::{Button, IconButton, ButtonMode}, card::Card},
    context::{AppDialog, AppDialogContent},
};
use anyhow::{bail, Result};
use leptos::{html::Dialog, leptos_dom::logging, *};

async fn delete(id: String) -> Result<()> {
    let res = reqwest::Client::new()
        .delete(&format!("http://localhost:6080/v1/cron/{}", id))
        .send()
        .await?;

    if res.status().is_success() {
        Ok(())
    } else {
        bail!("Request failed with status: {}", res.status())
    }
}

#[component]
fn CronJobDeleteDialog(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop(into)] refresh: Option<RwSignal<()>>,
) -> impl IntoView {
    let delete_action = create_action(|args: &(String, Option<RwSignal<()>>)| {
        let (id, refresh) = args.clone();
        async move {
            let _ = delete(id)
                .await
                .map_err(|e| logging::console_error(&e.to_string()));
            let _ = refresh.map(|x| x.set(()));
        }
    });
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 w-[500px] h-[300px]">
                <div class="grow">
                    "Are you sure you want to delete the cron job with id: "
                    {move || id.get()} "?"
                </div>
                <div class="flex items-stretch gap-x-4">
                    <Button on:click=move |_| {
                        delete_action.dispatch((id.get(), refresh));
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
pub fn CronJobDeleteButton(#[prop(into)] id: String) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let (id, _) = create_signal(id);
    let refresh = use_context::<RwSignal<()>>();

    view! {
        <IconButton
            icon="iconoir-bin-full"
            mode=ButtonMode::Danger
            on:click=move |_| {
                let Some(AppDialogContent(content)) = app_dialog_content else {
                    logging::console_error("App dialog content not found");
                    return;
                };

                let Some(AppDialog(dialog)) = app_dialog else {
                    logging::console_error("App dialog node ref not found");
                    return;
                };

                let _ = dialog.get().map(|x| x.show_modal());

                content.set(Some(view! {
                    <CronJobDeleteDialog id=id app_dialog=dialog refresh=refresh />
                }.into_view()));
            }/>
    }
}
