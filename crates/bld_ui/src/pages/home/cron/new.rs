use crate::components::{button::Button, card::Card, input::Input,
    list::{List, ListItem}
};
use anyhow::Result;
use bld_models::dtos::ListResponse;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;
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

fn into_list_items(close_dialog: WriteSignal<Option<()>>, data: Vec<ListResponse>) -> Vec<ListItem> {
    data.into_iter()
        .map(|x| {
            let pipeline_clone = x.pipeline.clone();
            ListItem {
                id: x.pipeline.clone(),
                content: Some(view! {
                    <button
                        class="w-full py-4 px-8 hover:bg-slate-600 hover:cursor-pointer flex items-center"
                        on:click=move |_| {
                            close_dialog.set(Some(()));
                            let nav = use_navigate();
                            nav(&format!("/cron/insert?name={}", pipeline_clone), NavigateOptions::default());
                        }>
                        {x.pipeline}
                    </button>
                }.into_view()),
                ..Default::default()
            }
        })
        .collect()
}

#[component]
fn CronJobsNewDialog(
    #[prop(into)] app_dialog: Option<NodeRef<Dialog>>,
) -> impl IntoView {
    let search = create_rw_signal(String::new());
    let (pipelines, set_pipelines) = create_signal(Vec::<ListItem>::new());
    let (close_dialog, set_close_dialog) = create_signal(None);

    let filtered_pipelines = move || {
        let search = search.get();
        if search.is_empty() {
            pipelines.get()
        } else {
            pipelines
                .get()
                .into_iter()
                .filter(|x| x.id.contains(&search))
                .collect()
        }
    };

    create_resource(
        move || (set_close_dialog, set_pipelines),
        |(set_close_dialog, set_pipelines)| async move {
            let data = get_pipelines()
                .await
                .map_err(|e| logging::console_log(&e.to_string()))
                .unwrap_or_default();

            set_pipelines.set(into_list_items(set_close_dialog, data));
        },
    );

    create_effect(move |_| {
        if close_dialog.get().is_some() {
            let _ = app_dialog.and_then(|x| x.get()).map(|x| x.close());
        }
    });

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-w-[600px] min-h-[600px]">
                <div class="text-xl">"Add new cron job"</div>
                <Input placeholder="Pipeline name" value=search />
                <div class="grow">
                    <List items=filtered_pipelines />
                </div>
                <Button on:click=move |_| {
                    let _ = app_dialog.and_then(|x| x.get()).map(|x| x.close());
                }>
                    "Close"
                </Button>
            </div>
        </Card>
    }
}

#[component]
pub fn CronJobsNewButton() -> impl IntoView {
    let app_dialog = use_context::<NodeRef<Dialog>>();
    let app_dialog_content = use_context::<RwSignal<Option<View>>>();

    view! {
        <Button on:click=move |_| {
            let Some(app_dialog_content) = app_dialog_content else {
                return;
            };
            app_dialog_content.set(Some(view! {
                <CronJobsNewDialog app_dialog=app_dialog />
            }));
            let _ = app_dialog.and_then(|x| x.get()).map(|x| x.show_modal());
        }>
            "Add new"
        </Button>
    }
}
