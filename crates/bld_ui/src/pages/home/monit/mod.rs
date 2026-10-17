mod artifacts;
mod logs;

use crate::{
    api::{self, build_ws_url, get_access_token},
    browser::save_bytes_as_file,
    components::{
        badge::RunState,
        button::{Button, ButtonVariant},
        link::BackLink,
        tabs::{Tab, Tabs},
    },
    context::use_app_dialog,
    format,
};
use bld_models::dtos::{HistQueryParams, HistoryEntry, MonitInfo};
use codee::string::FromToStringCodec;
use leptos::*;
use leptos_router::*;
use leptos_use::{
    ReconnectLimit, UseWebSocketOptions, UseWebSocketReturn, core::ConnectionReadyState,
    use_interval_fn, use_websocket_with_options,
};

use {
    artifacts::MonitArtifacts,
    logs::{LogStatus, MonitLogs},
};

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum MenuItem {
    #[default]
    Logs,
    Artifacts,
}

async fn get_run(id: Option<String>) -> Option<HistoryEntry> {
    let id = id?;
    let params = HistQueryParams {
        search: Some(id.clone()),
        limit: 5,
        ..Default::default()
    };
    api::hist(params)
        .await
        .ok()?
        .into_iter()
        .find(|r| r.id == id)
}

#[component]
fn MonitHeader(
    #[prop(into)] id: Signal<Option<String>>,
    #[prop(into)] run: Signal<Option<HistoryEntry>>,
    #[prop(into)] lines: Signal<Vec<String>>,
) -> impl IntoView {
    let dialog = use_app_dialog();
    let state = move || {
        run.get()
            .map(|r| RunState::parse(&r.state))
            .unwrap_or(RunState::NoRun)
    };
    let subtitle = move || {
        let Some(run) = run.get() else {
            return String::new();
        };
        let mut parts = vec![run.name.clone(), format::trigger_label(&run.user)];
        if let Some(start) = run.start_date_time.as_deref()
            && let Some(date) = format::parse_date(start)
        {
            parts.push(format!("started {}", date.format("%Y-%m-%d %H:%M:%S")));
        }
        let duration =
            format::run_duration(run.start_date_time.as_deref(), run.end_date_time.as_deref());
        if run.end_date_time.is_some() {
            parts.push(format!("took {duration}"));
        } else if run.start_date_time.is_some() {
            parts.push(format!("{} elapsed", duration.trim_end_matches('…')));
        }
        parts.join(" · ")
    };

    let stop_action = create_action(move |id: &String| {
        let id = id.clone();
        async move {
            if let Err(e) = api::stop(id).await {
                dialog.error(e.to_string());
            }
        }
    });

    let download = move |_| {
        let content = lines.get_untracked().join("\n");
        let name = format!(
            "{}.log",
            id.get_untracked().unwrap_or_else(|| "run".to_string())
        );
        if let Err(e) = save_bytes_as_file(content.as_bytes(), &name) {
            dialog.error(e.to_string());
        }
    };

    view! {
        <div class="flex items-start justify-between gap-5 flex-wrap">
            <div class="min-w-0">
                <BackLink href="/history">"History"</BackLink>
                <div class="flex items-center gap-[11px] flex-wrap">
                    <span class=move || {
                        format!("size-[7px] rounded-full {}", state().dot_class())
                    }></span>
                    <span
                        class="font-mono text-lg font-semibold tracking-[-.02em]"
                        title=move || id.get().unwrap_or_default()
                    >
                        {move || id.get().map(|i| format::short_id(&i)).unwrap_or_default()}
                    </span>
                    <span class=move || {
                        format!("text-xs {}", state().text_class())
                    }>{move || state().label()}</span>
                </div>
                <div class="text-ink-muted text-[12.5px] mt-1.5 font-mono break-words">
                    {subtitle}
                </div>
            </div>
            <div class="flex gap-[7px]">
                <Button variant=ButtonVariant::Ghost on:click=download>
                    "Download log"
                </Button>
                <Show
                    when=move || {
                        matches!(state(), RunState::Running | RunState::Queued | RunState::Initial)
                    }
                    fallback=|| view! {}
                >
                    <Button
                        variant=ButtonVariant::Danger
                        on:click=move |_| {
                            if let Some(id) = id.get_untracked() {
                                stop_action.dispatch(id);
                            }
                        }
                    >
                        "Stop run"
                    </Button>
                </Show>
            </div>
        </div>
    }
}

#[component]
pub fn Monit() -> impl IntoView {
    let params = use_query_map();
    let id = Signal::derive(move || params.with(|p| p.get("id").cloned()));
    let selected = create_rw_signal(MenuItem::Logs);
    let dialog = use_app_dialog();
    let (lines, set_lines) = create_signal(Vec::<String>::new());

    let run = create_resource(move || id.get(), get_run);
    let run_signal = Signal::derive(move || run.get().flatten());
    let done = move || {
        run_signal
            .get()
            .map(|r| RunState::parse(&r.state).is_done())
            .unwrap_or(false)
    };
    let _ = use_interval_fn(
        move || {
            if !done() {
                run.refetch();
            }
        },
        5_000,
    );

    let Ok(access_token) = get_access_token() else {
        return view! {}.into_view();
    };

    let mut path = "/v1/ws-monit/".to_string();
    if let Some(access_token) = access_token {
        path.push_str(&format!("?access_token={}", access_token));
    }

    let Ok(url) = build_ws_url(&path) else {
        return view! {}.into_view();
    };

    let UseWebSocketReturn {
        message,
        send,
        ready_state,
        ..
    } = use_websocket_with_options::<String, FromToStringCodec>(
        &url,
        UseWebSocketOptions::default()
            .reconnect_limit(ReconnectLimit::Limited(0))
            .on_error(move |e| dialog.error(e.to_string())),
    );

    create_effect(move |_| {
        if ready_state.get() == ConnectionReadyState::Open {
            let info = MonitInfo::new(id.get_untracked(), None, false);
            let body: String = serde_json::to_string(&info).unwrap_or_default();
            send(&body);
        }
    });

    create_effect(move |_| {
        if let Some(data) = message.get() {
            set_lines.update(|v| v.push(data));
        }
    });

    let status = Signal::derive(move || match ready_state.get() {
        ConnectionReadyState::Connecting => LogStatus::Connecting,
        ConnectionReadyState::Open if !done() => LogStatus::Following,
        _ => LogStatus::Ended,
    });

    view! {
        <MonitHeader id=id run=run_signal lines=lines />
        <Tabs>
            <Tab
                is_selected=Signal::derive(move || selected.get() == MenuItem::Logs)
                on:click=move |_| selected.set(MenuItem::Logs)
            >
                "Log"
            </Tab>
            <Tab
                is_selected=Signal::derive(move || selected.get() == MenuItem::Artifacts)
                on:click=move |_| selected.set(MenuItem::Artifacts)
            >
                "Artifacts"
            </Tab>
        </Tabs>
        <Show when=move || selected.get() == MenuItem::Logs fallback=|| view! {}>
            <MonitLogs lines=lines status=status />
        </Show>
        <Show when=move || selected.get() == MenuItem::Artifacts fallback=|| view! {}>
            <MonitArtifacts run_id=id />
        </Show>
    }
    .into_view()
}
