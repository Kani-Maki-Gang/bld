use crate::{
    api,
    components::button::{Button, ButtonSize, ButtonVariant, IconButton},
    context::{RefreshCronJobs, use_app_dialog},
    error::{DialogCard, ErrorBanner},
};
use leptos::*;

#[component]
fn CronJobDeleteDialog(#[prop()] id: String) -> impl IntoView {
    let dialog = use_app_dialog();
    let refresh = use_context::<RefreshCronJobs>();
    let error = create_rw_signal(None::<String>);

    let delete_action = create_action(move |id: &String| {
        let id = id.clone();
        async move {
            match api::cron_delete(id).await {
                Ok(_) => {
                    dialog.close();
                    if let Some(refresh) = refresh {
                        refresh.set();
                    }
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    view! {
        <DialogCard title="Delete cron job">
            <div class="mt-2 text-[12.5px] text-ink-soft leading-normal">
                "Delete cron job " <span class="font-mono text-ink break-all">{id.clone()}</span>
                "? You cannot undo this."
            </div>
            <Show when=move || error.get().is_some() fallback=|| view! {}>
                <div class="mt-3.5">
                    <ErrorBanner error=Signal::derive(move || error.get().unwrap_or_default()) />
                </div>
            </Show>
            <div class="mt-5 flex gap-2">
                <Button
                    variant=ButtonVariant::Danger
                    size=ButtonSize::Lg
                    class="flex-1"
                    on:click=move |_| delete_action.dispatch(id.clone())
                >
                    "Delete"
                </Button>
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Lg
                    class="flex-1"
                    on:click=move |_| dialog.close()
                >
                    "Cancel"
                </Button>
            </div>
        </DialogCard>
    }
}

#[component]
pub fn CronJobDeleteButton(#[prop()] id: String) -> impl IntoView {
    let dialog = use_app_dialog();
    view! {
        <IconButton
            variant=ButtonVariant::DangerGhost
            icon="iconoir-trash"
            title="Delete cron job"
            class="!size-[26px] !rounded-[7px]"
            on:click=move |_| {
                let id = id.clone();
                dialog.open(move || view! { <CronJobDeleteDialog id=id /> })
            }
        />
    }
}
