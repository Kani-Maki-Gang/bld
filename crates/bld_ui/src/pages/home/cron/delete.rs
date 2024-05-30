use crate::components::{button::Button, card::Card};
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
    #[prop(into)] app_dialog: Option<NodeRef<Dialog>>,
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
                    {id.get()} "?"
                </div>
                <div class="flex items-stretch gap-x-4">
                    <Button on:click=move |_| {
                        delete_action.dispatch((id.get(), refresh));
                        let _ = app_dialog.and_then(|x| x.get().map(|x| x.close()));
                    }>
                        "Delete"
                    </Button>
                    <Button on:click=move |_| {
                        let _ = app_dialog.and_then(|x| x.get().map(|x| x.close()));
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
    let app_dialog = use_context::<NodeRef<Dialog>>();
    let app_dialog_content = use_context::<RwSignal<Option<View>>>();
    let (id, _) = create_signal(id);
    let refresh = use_context::<RwSignal<()>>();

    view! {
        <button
            class="w-[30px] rounded-lg bg-red-500 text-xl grid place-items-center p-1"
            on:click=move |_| {
                let _ = app_dialog_content.map(|x| x.set(Some(view! {
                    <CronJobDeleteDialog id=id app_dialog=app_dialog refresh=refresh />
                }.into_view())));
                let _ = app_dialog.and_then(|x| x.get().map(|x| x.show_modal()));
            }>
                <i class="iconoir-bin-full" />
        </button>
    }
}