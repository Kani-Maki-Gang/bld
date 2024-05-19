mod info;
mod run;
mod v2;

use crate::components::{
    button::Button,
    card::Card,
    link::Link,
    table::{Table, TableRow},
};
use anyhow::Result;
use bld_models::dtos::ListResponse;
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

pub use info::PipelineInfo;
pub use run::{
    variables::RunPipelineVariables,
    RunPipeline,
};

async fn get_pipelines() -> Result<Vec<ListResponse>> {
    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/list")
        .header("Accept", "application/json")
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        Ok(vec![])
    }
}

fn into_table_rows(data: Vec<ListResponse>) -> Vec<TableRow> {
    data.into_iter()
        .map(|x| TableRow {
            columns: vec![
                view! {
                    <Link href={format!("/pipelines/info?id={}&name={}", x.id, x.pipeline)}>{x.id}</Link>
                }.into_view(),
                x.pipeline.into_view(),
            ]
        })
        .collect()
}

#[component]
pub fn Pipelines() -> impl IntoView {
    let (headers, _set_headers) = create_signal(vec!["Id".into_view(), "Name".into_view()]);

    let (rows, set_rows) = create_signal(vec![]);

    let list_res = create_resource(
        move || set_rows,
        |set_rows| async move {
            let data = get_pipelines()
                .await
                .map_err(|e| logging::console_log(e.to_string().as_str()))
                .unwrap_or_default();

            set_rows.set(into_table_rows(data));
        },
    );

    view! {
        <div class="flex flex-col gap-8 h-full">
            <Card>
                <div class="flex flex-col px-8 py-12">
                    <div class="flex justify-items-center gap-x-4 items-center">
                        <div class="grow flex flex-col">
                            <div class="text-2xl">
                                "Pipelines"
                            </div>
                            <div class="text-gray-400 mb-8">
                                "The list of all available pipelines"
                            </div>
                        </div>
                        <div class="w-40 flex items-end">
                            <Button on:click=move |_| list_res.refetch()>
                                "Refresh"
                            </Button>
                        </div>
                    </div>
                    <Table headers=headers rows=rows />
                </div>
            </Card>
        </div>
    }
}
