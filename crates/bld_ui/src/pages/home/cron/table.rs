use super::delete::CronJobDeleteButton;
use crate::components::{
    link::Link,
    table::{Body, Cell, Table, Header, Headers, Row},
};
use anyhow::Result;
use bld_models::dtos::{CronJobResponse, JobFiltersParams};
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

async fn get_cron(params: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
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

#[component]
pub fn CronJobsTable(#[prop(into)] params: Signal<Option<JobFiltersParams>>) -> impl IntoView {
    let (data, set_data) = create_signal(vec![]);
    let refresh = use_context::<RwSignal<()>>();

    let hist_res = create_resource(
        move || (params, set_data),
        |(params, set_data)| async move {
            let Some(params) = params.get_untracked() else {
                return;
            };

            let data = get_cron(&params)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .unwrap_or_default();

            set_data.set(data);
        },
    );

    let _ = watch(
        move || refresh.map(|x| x.get()),
        move |_, _, _| hist_res.refetch(),
        false,
    );

    view! {
        <Table>
            <Headers>
                <Header>"Id"</Header>
                <Header>"Pipeline"</Header>
                <Header>"Schedule"</Header>
                <Header>"Default"</Header>
                <Header>"Date created"</Header>
                <Header>"Date updated"</Header>
                <Header>""</Header>
            </Headers>
            <Body>
                <For
                    each=move || data
                        .get()
                        .into_iter()
                        .enumerate()
                        .map(|x| (x.0, x.1.id.clone(), x.1))
                    key=|(i, _, _)| *i
                    let:child>
                    <Row>
                        <Cell>
                            <Link href=format!("/cron/update?id={}", child.1)>
                                {child.1}
                            </Link>
                        </Cell>
                        <Cell>{child.2.pipeline}</Cell>
                        <Cell>{child.2.schedule}</Cell>
                        <Cell>{child.2.is_default}</Cell>
                        <Cell>{child.2.date_created}</Cell>
                        <Cell>{child.2.date_updated.unwrap_or_default()}</Cell>
                        <Cell>
                            <CronJobDeleteButton id=child.2.id />
                        </Cell>
                    </Row>
                </For>
            </Body>
        </Table>
    }
}
