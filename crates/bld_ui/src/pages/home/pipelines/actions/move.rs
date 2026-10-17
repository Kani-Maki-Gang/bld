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
fn PipelineMoveDialog(
    #[prop()] id: String,
    #[prop()] name: String,
    #[prop()] redirect: bool,
) -> impl IntoView {
    let dialog = use_app_dialog();
    let refresh = use_context::<RefreshPipelines>();
    let current = create_rw_signal(name.clone());
    let target = create_rw_signal(String::new());
    let error = create_rw_signal(None::<String>);

    let move_action = create_action(move |(pipeline, target): &(String, String)| {
        let params = PipelinePathRequest {
            pipeline: pipeline.clone(),
            target: target.clone(),
        };
        let id = id.clone();
        async move {
            let target = params.target.clone();
            match api::pipeline_move(params).await {
                Ok(_) => {
                    dialog.close();
                    if redirect {
                        let nav = use_navigate();
                        nav(
                            &format!("/files/info?id={id}&name={target}"),
                            NavigateOptions::default(),
                        );
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
            title="Rename pipeline"
            subtitle="The pipeline file moves to the new path on the server."
        >
            <DialogField label="Current" value=current disabled=true />
            <DialogField label="New" value=target placeholder="dev/new-name.yaml" />
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
                        move |_| move_action.dispatch((name.clone(), target.get_untracked()))
                    }
                >
                    "Rename"
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
pub fn PipelineMoveButton(
    #[prop(into)] id: Signal<String>,
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
                let (id, name) = (id.get_untracked(), name.get_untracked());
                dialog
                    .open(move || {
                        view! { <PipelineMoveDialog id=id name=name redirect=redirect /> }
                    })
            }
        >
            "Rename"
        </Button>
    }
}
