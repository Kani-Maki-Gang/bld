use crate::{
    components::{
        link::Link,
        table::{Body, Cell, Header, Headers, Row, Table},
    },
    context::RefreshPipelines,
};
use anyhow::Result;
use bld_models::dtos::ListResponse;
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
            </Headers>
            <Body>
                <For
                    each=move || data
                        .get()
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .map(|(i, x)| (i, x.pipeline.clone(), x))
                    key=move |(i, _, _)| *i
                    let:child>
                    <Row>
                        <Cell>
                            <Link href={format!("/pipelines/info?id={}&name={}", child.2.id, child.2.pipeline)}>
                                {child.2.id}
                            </Link>
                        </Cell>
                        <Cell>
                            {child.1}
                        </Cell>
                    </Row>
                </For>
            </Body>
        </Table>
    }
}
