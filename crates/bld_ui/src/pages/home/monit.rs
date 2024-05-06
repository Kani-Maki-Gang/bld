use crate::components::{badge::Badge, card::Card};
use leptos::*;
use leptos_router::*;
use leptos_use::{core::ConnectionReadyState, use_websocket, UseWebsocketReturn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonitInfo {
    id: Option<String>,
    pipeline: Option<String>,
    last: bool,
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

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="flex mb-8 gap-x-4">
                    <div class="grow text-2xl mb-4 ">
                        "Monitoring pipeline run"
                    </div>
                    <div class="flex-shrink">
                        <Badge><span class="font-bold">"Id: "</span>{id()}</Badge>
                    </div>
                    <div class="flex-shrink">
                        <Badge><span class="font-bold">"Socket state: "</span>{socket_state()}</Badge>
                    </div>
                </div>
                <div class="rounded-lg p-8 bg-gray-950 text-green-500 text-sm">
                    <For
                        each=move || history.get().into_iter().enumerate()
                        key=|(index, _)| *index
                        let:child>
                        <pre>
                            {child.1}
                        </pre>
                    </For>
                </div>
            </div>
        </Card>
    }
}
