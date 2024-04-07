use anyhow::Result;
use bld_models::dtos::{HistQueryParams, HistoryEntry};
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

use crate::components::table::{Table, TableRow};

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

fn into_table_rows(data: Vec<HistoryEntry>) -> Vec<TableRow> {
    data.into_iter()
        .map(|item| TableRow {
            columns: vec![
                item.id.into_view(),
                item.name.into_view(),
                item.user.into_view(),
                item.start_date_time.unwrap_or_default().into_view(),
                item.end_date_time.unwrap_or_default().into_view(),
                item.state.into_view(),
            ],
        })
        .collect()
}

#[component]
pub fn HistoryTable(
    #[prop(into)] params: Signal<HistQueryParams>,
    #[prop(into)] refresh: Signal<()>,
) -> impl IntoView {
    let (headers, _) = create_signal(vec![
        "Id".into_view(),
        "Name".into_view(),
        "User".into_view(),
        "Start Date".into_view(),
        "End Date".into_view(),
        "State".into_view(),
    ]);

    let (rows, set_rows) = create_signal(vec![]);

    let hist_res = create_resource(
        move || (params, set_rows),
        |(params, set_rows)| async move {
            let data = get_hist(&params.get_untracked())
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .unwrap_or_default();

            set_rows.set(into_table_rows(data));
        },
    );

    let _ = watch(
        move || refresh.get(),
        move |_, _, _| hist_res.refetch(),
        false,
    );

    view! {
        <div class="flex flex-col">
            <Table headers=headers rows=rows />
        </div>
    }
}
