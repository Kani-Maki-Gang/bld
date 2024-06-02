mod copy;
mod delete;

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
        },
    );

    let items = move || {
        data.get()
            .unwrap_or_default()
            .into_iter()
            .map(|x| {
                let (read, _) = create_signal(x);
                read
            })
    };

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
                    each=move || items()
                    key=move |r| r.get().id.clone()
                    let:child>
                    <Row>
                        <Cell>
                            <Link href=child.with_untracked(|c| format!("/pipelines/info?id={}&name={}", c.id, c.pipeline))>
                                {move || child.get_untracked().id}
                            </Link>
                        </Cell>
                        <Cell>
                            {move || child.get_untracked().pipeline}
                        </Cell>
                        <Cell>
                            <div class="flex gap-2">
                                <PipelineTableCopyButton name=move || child.get_untracked().pipeline/>
                                <PipelineTableDeleteButton name=move || child.get_untracked().pipeline/>
                            </div>
                        </Cell>
                    </Row>
                </For>
            </Body>
        </Table>
    }
}
