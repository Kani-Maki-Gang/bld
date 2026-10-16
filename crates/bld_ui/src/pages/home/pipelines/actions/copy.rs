use super::DialogField;
use crate::{
    api,
    components::button::{Button, ButtonSize, ButtonVariant},
    context::{RefreshPipelines, use_app_dialog},
    error::{DialogCard, ErrorBanner},
};
use bld_models::dtos::PipelinePathRequest;
use leptos::*;
use leptos_router::*;

#[component]
fn PipelineCopyDialog(#[prop()] name: String, #[prop()] redirect: bool) -> impl IntoView {
    let dialog = use_app_dialog();
    let refresh = use_context::<RefreshPipelines>();
    let source = create_rw_signal(name.clone());
    let target = create_rw_signal(String::new());
    let error = create_rw_signal(None::<String>);

    let copy_action = create_action(move |(pipeline, target): &(String, String)| {
        let params = PipelinePathRequest {
            pipeline: pipeline.clone(),
            target: target.clone(),
        };
        async move {
            match api::copy(params).await {
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
        <DialogCard
            title="Copy pipeline"
            subtitle="Creates a new pipeline file with the same content."
        >
            <DialogField label="Source" value=source disabled=true />
            <DialogField label="Copy to" value=target placeholder="dev/copy.yaml" />
            <Show when=move || error.get().is_some() fallback=|| view! {}>
                <div class="mt-3.5">
                    <ErrorBanner error=Signal::derive(move || error.get().unwrap_or_default()) />
                </div>
            </Show>
            <div class="mt-5 flex gap-2">
                <Button
                    size=ButtonSize::Lg
                    class="flex-1"
                    on:click={
                        let name = name.clone();
                        move |_| copy_action.dispatch((name.clone(), target.get_untracked()))
                    }
                >
                    "Copy"
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
pub fn PipelineCopyButton(
    #[prop(into)] name: Signal<String>,
    #[prop(optional)] redirect: bool,
    #[prop(optional)] size: ButtonSize,
) -> impl IntoView {
    let dialog = use_app_dialog();
    view! {
        <Button
            variant=ButtonVariant::Ghost
            size=size
            on:click=move |_| {
                let name = name.get_untracked();
                dialog.open(move || view! { <PipelineCopyDialog name=name redirect=redirect /> })
            }
        >
            "Copy"
        </Button>
    }
}
