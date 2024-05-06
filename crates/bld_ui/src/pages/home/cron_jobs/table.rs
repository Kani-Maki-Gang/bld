use anyhow::Result;
use bld_models::dtos::{JobFiltersParams, CronJobResponse};
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

use crate::components::table::{Table, TableRow};

async fn get_hist(params: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/cron")
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

fn into_table_rows(data: Vec<CronJobResponse>) -> Vec<TableRow> {
    data.into_iter()
        .map(|item| TableRow {
            columns: vec![
                item.id.into_view(),
                item.pipeline.into_view(),
                item.schedule.into_view(),
                item.is_default.into_view(),
                item.date_created.into_view(),
                item.date_updated.unwrap_or_default().into_view(),
            ],
        })
        .collect()
}

#[component]
pub fn CronJobsTable(
    #[prop(into)] params: Signal<Option<JobFiltersParams>>,
    #[prop(into)] refresh: Signal<()>,
) -> impl IntoView {
    let (headers, _) = create_signal(vec![
        "Id".into_view(),
        "Pipeline".into_view(),
        "Schedule".into_view(),
        "Default".into_view(),
        "Date created".into_view(),
        "Date updated".into_view(),
    ]);

    let (rows, set_rows) = create_signal(vec![]);

    let hist_res = create_resource(
        move || (params, set_rows),
        |(params, set_rows)| async move {
            let Some(params) = params.get_untracked() else {
                return;
            };

            let data = get_hist(&params)
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
        <Table headers=headers rows=rows />
    }
}
