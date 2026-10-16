use crate::{
    api,
    components::button::{Button, ButtonSize, ButtonVariant, IconButton},
    context::{RefreshPipelines, use_app_dialog},
    error::{DialogCard, ErrorBanner},
};
use bld_models::dtos::PipelineQueryParams;
use leptos::*;
use leptos_router::*;

#[component]
fn PipelineDeleteDialog(#[prop()] name: String, #[prop()] redirect: bool) -> impl IntoView {
    let dialog = use_app_dialog();
    let refresh = use_context::<RefreshPipelines>();
    let error = create_rw_signal(None::<String>);

    let delete_action = create_action(move |pipeline: &String| {
        let params = PipelineQueryParams {
            pipeline: pipeline.clone(),
        };
        async move {
            match api::remove(params).await {
                Ok(_) => {
                    dialog.close();
                    if redirect {
                        let nav = use_navigate();
                        nav("/files", NavigateOptions::default());
                    } else if let Some(refresh) = refresh {
                        refresh.set();
                    }
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    view! {
        <DialogCard title="Delete pipeline">
            <div class="mt-2 text-[12.5px] text-ink-soft leading-normal">
                "Delete " <span class="font-mono text-ink">{name.clone()}</span>
                " and its cron jobs? You cannot undo this."
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
                    on:click={
                        let name = name.clone();
                        move |_| delete_action.dispatch(name.clone())
                    }
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
pub fn PipelineDeleteButton(
    #[prop(into)] name: Signal<String>,
    #[prop(optional)] redirect: bool,
    #[prop(optional)] icon: bool,
    #[prop(optional)] size: ButtonSize,
) -> impl IntoView {
    let dialog = use_app_dialog();
    let open = move |_| {
        let name = name.get_untracked();
        dialog.open(move || view! { <PipelineDeleteDialog name=name redirect=redirect /> })
    };
    if icon {
        view! {
            <IconButton
                variant=ButtonVariant::DangerGhost
                icon="iconoir-trash"
                title="Delete pipeline"
                on:click=open
            />
        }
        .into_view()
    } else {
        view! {
            <Button variant=ButtonVariant::DangerGhost size=size on:click=open>
                "Delete"
            </Button>
        }
        .into_view()
    }
}
