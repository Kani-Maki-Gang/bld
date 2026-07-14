use crate::{
    api,
    components::{
        button::{Button, IconButton},
        card::Card,
        colors::Colors,
    },
    context::{AppDialog, AppDialogContent, RefreshArtifacts},
    error::SmallError,
};
use leptos::{html::Dialog, leptos_dom::logging, *};

type DeleteActionArgs = (
    String,
    RwSignal<Option<String>>,
    Option<RefreshArtifacts>,
    NodeRef<Dialog>,
);

#[component]
fn ArtifactDeleteDialog(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshArtifacts>,
) -> impl IntoView {
    let error = create_rw_signal(None);

    let delete_action = create_action(|args: &DeleteActionArgs| {
        let (id, error, refresh, dialog) = args.clone();
        async move {
            match api::artifact_delete(id).await {
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
        <Card class="px-8 py-10 gap-6 w-[480px]">
            <div class="grow text-sm text-zinc-300">
                "Are you sure you want to delete artifact "
                <span class="font-medium text-white">{move || name.get()}</span>
                "? This action cannot be undone."
            </div>
            <Show when=move || error.get().is_some() fallback=|| view! {}>
                <SmallError error=move || error.get().unwrap() />
            </Show>
            <div class="flex gap-3">
                <Button
                    color=Colors::Red
                    on:click=move |_| {
                        delete_action.dispatch((id.get(), error, refresh, app_dialog));
                    }
                >
                    "Delete"
                </Button>
                <Button
                    ghost=true
                    on:click=move |_| {
                        let _ = app_dialog.get().map(|x| x.close());
                    }
                >
                    "Cancel"
                </Button>
            </div>
        </Card>
    }
}

#[component]
pub fn ArtifactDeleteButton(#[prop(into)] id: String, #[prop(into)] name: String) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let refresh = use_context::<RefreshArtifacts>();
    let (id, _) = create_signal(id);
    let (name, _) = create_signal(name);

    view! {
        <IconButton
            icon="iconoir-bin-full"
            color=Colors::Red
            ghost=true
            on:click=move |_| {
                let Some(AppDialog(dialog)) = app_dialog else {
                    logging::console_error("App dialog context not found");
                    return;
                };
                let Some(AppDialogContent(content)) = app_dialog_content else {
                    logging::console_error("App dialog context not found");
                    return;
                };
                content
                    .set(
                        Some(
                            view! {
                                <ArtifactDeleteDialog
                                    id=id
                                    name=name
                                    app_dialog=dialog
                                    refresh=refresh
                                />
                            },
                        ),
                    );
                let _ = dialog.get().map(|x| x.show_modal());
            }
        />
    }
}
