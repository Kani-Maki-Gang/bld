use crate::{
    api,
    components::{
        badge::Badge,
        link::Link,
        table::{Body, Cell, Header, Headers, Row, Table},
    },
    context::RefreshHistory,
    error::Error,
};
use anyhow::{anyhow, Result};
use bld_models::dtos::{HistQueryParams, HistoryEntry};
use leptos::{leptos_dom::logging, *};
use leptos_use::signal_debounced;

async fn get_hist(params: Option<HistQueryParams>) -> Result<Vec<HistoryEntry>> {
    let params = params.ok_or_else(|| anyhow!("No query params provided for /v1/hist request"))?;
    api::hist(params).await
}

#[component]
pub fn HistoryEntryState(#[prop(into)] state: String) -> impl IntoView {
    let (icon, label, class) = match state.as_str() {
        "initial" => ("iconoir-running", "Intial", "bg-yellow-600"),
        "queued" => ("iconoir-clock", "Queued", ""),
        "running" => ("iconoir-running", "Running", ""),
        "finished" => ("iconoir-check-circle", "Finished", "bg-emerable-600"),
        "faulted" => ("iconoir-minus-circle", "Faulted", "bg-red-600"),
        _ => ("", "Unknown", "bg-black"),
    };

    let icon = format!("{icon} mr-2");

    view! {
        <div class="w-28">
            <Badge class=class>
                <div class="flex items-center">
                    <i class=icon></i>
                    {label}
                </div>
            </Badge>
        </div>
    }
}

#[component]
pub fn HistoryTable(#[prop(into)] params: Signal<Option<HistQueryParams>>) -> impl IntoView {
    let refresh = use_context::<RefreshHistory>();
    let params_debounced = signal_debounced(params, 500.0);

    let data = create_resource(
        move || params_debounced.get(),
        |params| async move { get_hist(params).await.map_err(|e| e.to_string()) },
    );

    let _ = watch(
        move || {
            if let Some(RefreshHistory(refresh)) = refresh {
                refresh.get();
            } else {
                logging::console_error("Refresh history signal not found in context");
            }
        },
        move |_, _, _| data.refetch(),
        false,
    );

    view! {
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <Error error=move || data.get().unwrap().unwrap_err()/>
        </Show>
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
            <Table>
                <Headers>
                    <Header>"Id"</Header>
                    <Header>"Name"</Header>
                    <Header>"User"</Header>
                    <Header>"Start Date"</Header>
                    <Header>"End Date"</Header>
                    <Header>"State"</Header>
                </Headers>
                <Body>
                    <For
                        each=move || data.get().unwrap().unwrap().into_iter()
                        key=move |e| e.id.clone()
                        let:child
                    >
                        <Row>
                            <Cell>
                                <Link href=format!("/monit?id={}", child.id)>{child.id}</Link>
                            </Cell>
                            <Cell>{child.name}</Cell>
                            <Cell>{child.user}</Cell>
                            <Cell>{child.start_date_time.unwrap_or_default()}</Cell>
                            <Cell>{child.end_date_time.unwrap_or_default()}</Cell>
                            <Cell>
                                <HistoryEntryState state=child.state/>
                            </Cell>
                        </Row>
                    </For>
                </Body>
            </Table>
        </Show>
    }
}
