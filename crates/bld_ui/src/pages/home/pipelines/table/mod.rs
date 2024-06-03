mod copy;
mod delete;
mod r#move;
mod run;

use crate::{
    components::{
        link::Link, table::{Body, Cell, Header, Headers, Row, Table}
    },
    context::RefreshPipelines,
};
use anyhow::Result;
use bld_models::dtos::ListResponse;
use copy::PipelineTableCopyButton;
use delete::PipelineTableDeleteButton;
use r#move::PipelineTableMoveButton;
use run::PipelineTableRunButton;
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

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

#[component]
pub fn PipelinesTable() -> impl IntoView {
    let refresh = use_context::<RefreshPipelines>();

    let data = create_resource(
        || (),
        |_| async move {
            get_pipelines()
                .await
                .map_err(|e| logging::console_log(e.to_string().as_str()))
                .unwrap_or_default()
                .into_iter()
                .map(|x| create_rw_signal(x))
                .collect::<Vec<RwSignal<ListResponse>>>()
        },
    );

    let _ = watch(
        move || refresh.map(|x| x.get()),
        move |_, _, _| data.refetch(),
        false,
    );

    view! {
        <Table>
            <Headers>
                <Header>"Id"</Header>
                <Header>"Name"</Header>
                <Header>"Actions"</Header>
            </Headers>
            <Body>
                <For
                    each=move || data.get().unwrap_or_default()
                    key=move |r| r.get().pipeline.clone()
                    let:child>
                    <Row>
                        <Cell>
                            <Link href=child.with_untracked(|c| format!("/pipelines/info?id={}&name={}", c.id, c.pipeline))>
                                {move || child.get().id}
                            </Link>
                        </Cell>
                        <Cell>
                            {move || child.get().pipeline}
                        </Cell>
                        <Cell>
                            <div class="flex gap-2">
                                <PipelineTableRunButton id=move || child.get().id name=move || child.get().pipeline/>
                                <PipelineTableMoveButton name=move || child.get().pipeline/>
                                <PipelineTableCopyButton name=move || child.get().pipeline/>
                                <PipelineTableDeleteButton name=move || child.get().pipeline/>
                            </div>
                        </Cell>
                    </Row>
                </For>
            </Body>
        </Table>
    }
}
