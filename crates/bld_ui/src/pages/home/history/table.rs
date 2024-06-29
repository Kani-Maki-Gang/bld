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
use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{HistQueryParams, HistoryEntry};
use leptos::{leptos_dom::logging, *};

async fn get_hist(params: Option<HistQueryParams>) -> Result<Vec<HistoryEntry>> {
    let params = params.ok_or_else(|| anyhow!("No query params provided for /v1/hist request"))?;
    let res = api::hist(params).await?;
    let status = res.status();
    if status.is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        let body = res.text().await?;
        let error = format!("Status {status} {body}");
        logging::console_error(&error);
        bail!(error)
    }
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

    let data = create_resource(
        move || params.get(),
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
                        each=move || data.get().unwrap().unwrap().into_iter().enumerate()
                        key=move |(i, _)| *i
                        let:child
                    >
                        <Row>
                            <Cell>
                                <Link href=format!("/monit?id={}", child.1.id)>{child.1.id}</Link>
                            </Cell>
                            <Cell>{child.1.name}</Cell>
                            <Cell>{child.1.user}</Cell>
                            <Cell>{child.1.start_date_time.unwrap_or_default()}</Cell>
                            <Cell>{child.1.end_date_time.unwrap_or_default()}</Cell>
                            <Cell>
                                <HistoryEntryState state=child.1.state/>
                            </Cell>
                        </Row>
                    </For>
                </Body>
            </Table>
        </Show>
    }
}
