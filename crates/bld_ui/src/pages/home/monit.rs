use crate::{
    components::{badge::Badge, button::Button, card::Card, colors::Colors},
    context::{AppDialog, AppDialogContent},
    error::ErrorDialog,
};
use anyhow::{bail, Result};
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
use leptos_use::{core::ConnectionReadyState, use_websocket, UseWebsocketReturn};
use reqwest::Client;
use serde::{Deserialize, Serialize};

type StopActionArgs = (String, NodeRef<Dialog>, RwSignal<Option<View>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonitInfo {
    id: Option<String>,
    pipeline: Option<String>,
    last: bool,
}

async fn stop(id: String) -> Result<()> {
    let res = Client::builder()
        .build()?
        .post("http://localhost:6080/v1/stop")
        .json(&id)
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
pub fn Monit() -> impl IntoView {
    let params = use_query_map();
    let id = move || params.with(|p| p.get("id").cloned());
    let info = move || MonitInfo {
        id: id(),
        pipeline: None,
        last: false,
    };
    let (history, set_history) = create_signal(vec![]);
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();

    let UseWebsocketReturn {
        message,
        send,
        ready_state,
        ..
    } = use_websocket("ws://localhost:6080/v1/ws-monit/");

    let socket_state = move || match ready_state.get() {
        ConnectionReadyState::Connecting => "Connecting",
        ConnectionReadyState::Open => "Open",
        ConnectionReadyState::Closing => "Closing",
        ConnectionReadyState::Closed => "Closed",
    };

    create_effect(move |_| {
        if ready_state.get() == ConnectionReadyState::Open {
            let info = info();
            send(serde_json::to_string(&info).unwrap_or_default().as_str());
        }
    });

    create_effect(move |_| {
        if let Some(data) = message.get() {
            set_history.update(|v: &mut Vec<String>| v.push(data));
        }
    });

    let stop_action = create_action(|args: &StopActionArgs| {
        let (id, dialog, content) = args.clone();
        async move {
            let res = stop(id).await;
            if let Err(e) = res {
                content.set(Some(
                    view! { <ErrorDialog dialog=dialog error=move || e.to_string()/> },
                ));
                let _ = dialog.get().map(|x| x.show_modal());
            }
        }
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4">
                <div class="flex mb-8 gap-x-4 items-start">
                    <div class="grow flex flex-col">
                        <div class="text-2xl">"Monitoring pipeline run"</div>
                        <div class="text-gray-400">
                            "Currently monitoring pipeline run with id: " {move || id()}
                        </div>
                    </div>
                    <div class="flex-shrink flex flex-row gap-2 items-center">
                        <div class="w-auto">
                            <Badge>
                                <span class="fs-bold">"State: "</span>
                                {move || socket_state}
                            </Badge>
                        </div>
                        <div class="w-32">
                            <Button
                                color=Colors::Red
                                on:click=move |_| {
                                    let Some(id) = id() else {
                                        logging::console_error(
                                            "Pipeline run id not provided in url",
                                        );
                                        return;
                                    };
                                    let Some(AppDialog(dialog)) = app_dialog else {
                                        logging::console_error("App dialog context not found");
                                        return;
                                    };
                                    let Some(AppDialogContent(content)) = app_dialog_content else {
                                        logging::console_error("App dialog context not found");
                                        return;
                                    };
                                    stop_action.dispatch((id, dialog, content));
                                }
                            >

                                "Stop"
                            </Button>
                        </div>
                    </div>
                </div>
                <div class="rounded-lg p-8 bg-gray-950 text-green-500 text-sm">
                    <For
                        each=move || history.get().into_iter().enumerate()
                        key=|(index, _)| *index
                        let:child
                    >
                        <pre>{child.1}</pre>
                    </For>
                </div>
            </div>
        </Card>
    }
}
