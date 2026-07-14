mod delete;
mod download;

use crate::{
    api,
    components::{
        button::IconButton,
        colors::Colors,
        table::{Body, Cell, Header, Headers, Row, Table},
    },
    context::RefreshArtifacts,
    error::Error,
};
use anyhow::Result;
use bld_models::dtos::{ArtifactResponse, ArtifactsQueryParams};
use leptos::{leptos_dom::logging, *};

use {delete::ArtifactDeleteButton, download::ArtifactDownloadButton};

async fn get_artifacts(run_id: Option<String>) -> Result<Vec<ArtifactResponse>> {
    let run_id = run_id.ok_or_else(|| anyhow::anyhow!("Run id not provided"))?;
    api::artifacts(ArtifactsQueryParams { run_id }).await
}

#[component]
pub fn MonitArtifacts(#[prop(into)] run_id: Signal<Option<String>>) -> impl IntoView {
    let refresh = use_context::<RefreshArtifacts>();

    let data = create_resource(
        move || run_id.get(),
        |run_id| async move { get_artifacts(run_id).await.map_err(|e| e.to_string()) },
    );

    let _ = watch(
        move || {
            if let Some(RefreshArtifacts(refresh)) = refresh {
                refresh.get();
            } else {
                logging::console_error("Refresh artifacts signal not found in context");
            }
        },
        move |_, _, _| data.refetch(),
        false,
    );

    view! {
        <div class="px-6 py-5 grow flex flex-col gap-4">
            <div class="flex justify-end">
                <IconButton
                    icon="iconoir-refresh-double"
                    ghost=true
                    color=Colors::Violet
                    on:click=move |_| {
                        let Some(refresh) = refresh else {
                            logging::console_error("Refresh artifacts signal not found in context");
                            return;
                        };
                        refresh.set();
                    }
                />
            </div>
            <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
                <Error error=move || data.get().unwrap().unwrap_err() />
            </Show>
            <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
                <Table>
                    <Headers>
                        <Header>"Id"</Header>
                        <Header>"Name"</Header>
                        <Header>"Date created"</Header>
                        <Header>"Date expires"</Header>
                        <Header>"Actions"</Header>
                    </Headers>
                    <Body>
                        <For
                            each=move || data.get().unwrap().unwrap().into_iter()
                            key=|e| e.id.clone()
                            let:child
                        >
                            {
                                let id = child.id;
                                let name = child.name.clone();
                                let display_name = name.clone();
                                let display_id = id.clone();
                                let download_id = id.clone();
                                view! {
                                    <Row>
                                        <Cell>{display_id}</Cell>
                                        <Cell>{display_name}</Cell>
                                        <Cell>{child.date_created}</Cell>
                                        <Cell>{child.date_expires}</Cell>
                                        <Cell>
                                            <div class="flex gap-2">
                                                <ArtifactDownloadButton id=download_id name=name.clone() />
                                                <ArtifactDeleteButton id=id name=name />
                                            </div>
                                        </Cell>
                                    </Row>
                                }
                            }
                        </For>
                    </Body>
                </Table>
            </Show>
        </div>
    }
}
