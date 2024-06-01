use crate::{
    components::{
        badge::Badge,
        link::Link,
        table::{Body, Cell, Header, Headers, Row, Table},
    },
    context::RefreshHistory,
};
use anyhow::Result;
use bld_models::dtos::{HistQueryParams, HistoryEntry};
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

async fn get_hist(params: &HistQueryParams) -> Result<Vec<HistoryEntry>> {
    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/hist")
        .query(params)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        Ok(vec![])
    }
}

#[component]
pub fn HistoryEntryState(#[prop(into)] state: String) -> impl IntoView {
    let (icon, label, class) = match state.as_str() {
        "initial" => ("iconoir-running", "Intial", "bg-yellow-600"),
        "queued" => ("iconoir-clock", "Queued", ""),
        "running" => ("iconoir-running", "Running", ""),
        "finished" => (
            "iconoir-check-circle",
            "Finished",
            "bg-emerable-600",
        ),
        "faulted" => ("iconoir-minus-circle", "Faulted", "bg-red-600"),
        _ => ("", "Unknown", "bg-black"),
    };

    let icon = format!("{icon} mr-2");

    view! {
        <div class="w-28">
            <Badge class=class>
                <div class="flex items-center">
                    <i class=icon></i>{label}
                </div>
            </Badge>
        </div>
    }
}

#[component]
pub fn HistoryTable(#[prop(into)] params: Signal<Option<HistQueryParams>>) -> impl IntoView {
    let (data, set_data) = create_signal(vec![]);
    let refresh = use_context::<RefreshHistory>();

    let hist_res = create_resource(
        move || (params, set_data),
        |(params, set_data)| async move {
            let Some(params) = params.get_untracked() else {
                return;
            };

            let data = get_hist(&params)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .unwrap_or_default();

            set_data.set(data);
        },
    );

    let _ = watch(
        move || {
            if let Some(RefreshHistory(refresh)) = refresh {
                refresh.get();
            } else {
                logging::console_error("Refresh history signal not found in context");
            }
        },
        move |_, _, _| hist_res.refetch(),
        false,
    );

    view! {
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
                    each=move || data.get().into_iter().enumerate()
                    key=move |(i, _)| *i
                    let:child>
                    <Row>
                        <Cell>
                            <Link href=format!("/monit?id={}", child.1.id)>{child.1.id}</Link>
                        </Cell>
                        <Cell>{child.1.name}</Cell>
                        <Cell>{child.1.user}</Cell>
                        <Cell>{child.1.start_date_time.unwrap_or_default()}</Cell>
                        <Cell>{child.1.end_date_time.unwrap_or_default()}</Cell>
                        <Cell>
                            <HistoryEntryState state=child.1.state />
                        </Cell>
                    </Row>
                </For>
            </Body>
        </Table>
    }
}
