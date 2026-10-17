use crate::{
    api,
    components::button::{Button, ButtonSize, ButtonVariant, IconButton},
    context::{RefreshArtifacts, use_app_dialog},
    error::{DialogCard, ErrorBanner},
};
use leptos::*;

#[component]
fn ArtifactDeleteDialog(#[prop()] id: String, #[prop()] name: String) -> impl IntoView {
    let dialog = use_app_dialog();
    let refresh = use_context::<RefreshArtifacts>();
    let error = create_rw_signal(None::<String>);

    let delete_action = create_action(move |id: &String| {
        let id = id.clone();
        async move {
            match api::artifact_delete(id).await {
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
        <DialogCard title="Delete artifact">
            <div class="mt-2 text-[12.5px] text-ink-soft leading-normal">
                "Delete " <span class="font-mono text-ink">{name}</span> "? You cannot undo this."
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
pub fn ArtifactDeleteButton(#[prop()] id: String, #[prop()] name: String) -> impl IntoView {
    let dialog = use_app_dialog();
    view! {
        <IconButton
            variant=ButtonVariant::DangerGhost
            icon="iconoir-trash"
            title="Delete artifact"
            on:click=move |_| {
                let (id, name) = (id.clone(), name.clone());
                dialog.open(move || view! { <ArtifactDeleteDialog id=id name=name /> })
            }
        />
    }
}
