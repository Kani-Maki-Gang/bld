use crate::{
    api,
    components::{
        button::{Button, IconButton},
        card::Card,
        colors::Colors,
    },
    context::{AppDialog, AppDialogContent, RefreshPipelines},
    error::SmallError,
};
use bld_models::dtos::PipelineQueryParams;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;

type DeleteActionArgs = (
    String,
    RwSignal<Option<String>>,
    Option<RefreshPipelines>,
    bool,
    NodeRef<Dialog>,
);

#[component]
fn PipelineDeleteButtonDialog(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] app_dialog: NodeRef<Dialog>,
    #[prop()] refresh: Option<RefreshPipelines>,
    #[prop(into, optional)] redirect: bool,
) -> impl IntoView {
    let error = create_rw_signal(None);
    let delete_action = create_action(|args: &DeleteActionArgs| {
        let (name, error, refresh, redirect, dialog) = args.clone();
        async move {
            let params = PipelineQueryParams { pipeline: name };
            match api::remove(params).await {
                Ok(_) if redirect => {
                    let nav = use_navigate();
                    nav("/pipelines", NavigateOptions::default());
                    let _ = dialog.get().map(|x| x.close());
                }
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
                "Are you sure you want to delete "
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
                        delete_action.dispatch((name.get(), error, refresh, redirect, app_dialog));
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
pub fn PipelineDeleteButton(
    #[prop(into)] name: Signal<String>,
    #[prop(into, optional)] redirect: bool,
) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();
    let refresh = use_context::<RefreshPipelines>();
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
