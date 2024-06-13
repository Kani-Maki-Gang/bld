use crate::{
    components::{
        button::{Button, IconButton},
        card::Card,
        colors::Colors,
    },
    context::{AppDialog, AppDialogContent, RefreshCronJobs},
    error::SmallError,
};
use anyhow::{bail, Result};
use leptos::{html::Dialog, leptos_dom::logging, *};

type DeleteActionArgs = (
    String,
    RwSignal<Option<String>>,
    NodeRef<Dialog>,
    Option<RefreshCronJobs>,
);

async fn delete(id: String) -> Result<()> {
    let res = reqwest::Client::new()
        .delete(&format!("http://localhost:6080/v1/cron/{}", id))
        .send()
        .await?;

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
fn CronJobDeleteDialog(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop(into)] refresh: Option<RefreshCronJobs>
) -> impl IntoView {
    let error = create_rw_signal(None);

    let delete_action = create_action(|args: &DeleteActionArgs| {
        let (id, error, dialog, refresh) = args.clone();
        async move {
            match delete(id).await {
                Ok(_) => {
                    let _ = dialog.get().map(|x| x.close());
                    let _ = refresh.map(|x| x.set());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        }
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4 w-[500px] h-[300px]">
                <div class="grow">
                    "Are you sure you want to delete the cron job with id: " {move || id.get()} "?"
                </div>
                <Show when=move || error.get().is_some() fallback=|| view! {}>
                    <SmallError error=move || error.get().unwrap()/>
                </Show>
                <div class="flex items-stretch gap-x-4">
                    <Button on:click=move |_| {
                        delete_action.dispatch((id.get(), error, app_dialog, refresh));
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
pub fn CronJobDeleteButton(#[prop(into)] id: String) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let (id, _) = create_signal(id);
    let refresh = use_context::<RefreshCronJobs>();

    view! {
        <IconButton
            icon="iconoir-bin-full"
            color=Colors::Red
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
                content
                    .set(
                        Some(
                            view! { <CronJobDeleteDialog id=id app_dialog=dialog refresh=refresh/> }
                                .into_view(),
                        ),
                    );
            }
        />
    }
}
