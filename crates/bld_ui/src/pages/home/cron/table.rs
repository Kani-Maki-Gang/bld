use super::delete::CronJobDeleteButton;
use crate::{
    api,
    components::{
        link::Link,
        table::{Body, Cell, Header, Headers, Row, Table},
    },
    context::RefreshCronJobs,
    error::Error,
};
use anyhow::Result;
use bld_models::dtos::{CronJobResponse, JobFiltersParams};
use leptos::{leptos_dom::logging, *};
use leptos_use::signal_debounced;

async fn get_cron(params: Option<JobFiltersParams>) -> Result<Vec<CronJobResponse>> {
    let params =
        params.ok_or_else(|| anyhow::anyhow!("Params not provided for /v1/cron request"))?;
    api::cron(params).await
}

#[component]
pub fn CronJobsTable(#[prop(into)] params: Signal<Option<JobFiltersParams>>) -> impl IntoView {
    let refresh = use_context::<RefreshCronJobs>();
    let params_debounced = signal_debounced(params, 500.0);

    let data = create_resource(
        move || params_debounced.get(),
        |params| async move { get_cron(params).await.map_err(|e| e.to_string()) },
    );

    let _ = watch(
        move || {
            if let Some(RefreshCronJobs(refresh)) = refresh {
                refresh.get();
            } else {
                logging::console_error("Refresh cron jobs signal not found in context");
            }
        },
        move |_, _, _| data.refetch(),
        false,
    );

    view! {
        <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
            <Error error=move || data.get().unwrap().unwrap_err() />
        </Show>
        <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
            <Table>
                <Headers>
                    <Header>"Id"</Header>
                    <Header>"Pipeline"</Header>
                    <Header>"Schedule"</Header>
                    <Header>"Default"</Header>
                    <Header>"Date created"</Header>
                    <Header>"Date updated"</Header>
                    <Header>"Actions"</Header>
                </Headers>
                <Body>
                    <For
                        each=move || data.get().unwrap().unwrap().into_iter()
                        key=move |e| e.id.clone()
                        let:child
                    >
                        {
                            let id = child.id.clone();
                            view! {
                                <Row>
                                    <Cell>
                                        <Link href=format!(
                                            "/cron/update?id={id}",
                                        )>{id.clone()}</Link>
                                    </Cell>
                                    <Cell>{child.pipeline}</Cell>
                                    <Cell>{child.schedule}</Cell>
                                    <Cell>{child.is_default}</Cell>
                                    <Cell>{child.date_created}</Cell>
                                    <Cell>{child.date_updated.unwrap_or_default()}</Cell>
                                    <Cell>
                                        <CronJobDeleteButton id=child.id />
                                    </Cell>
                                </Row>
                            }
                        }
                    </For>
                </Body>
            </Table>
        </Show>
    }
}
