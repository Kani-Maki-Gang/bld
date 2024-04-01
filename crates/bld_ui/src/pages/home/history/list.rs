use anyhow::Result;
use bld_models::dtos::{HistQueryParams, HistoryEntry};
use leptos::{leptos_dom::logging, *};
use reqwest::Client;

use crate::components::{
    list::{List, ListItem},
    table::{Table, TableRow},
};

fn get_params(
    state: Option<String>,
    limit: Option<String>,
    pipeline: Option<String>,
) -> HistQueryParams {
    HistQueryParams {
        name: pipeline,
        state: state.filter(|x| x != "all"),
        limit: limit
            .as_ref()
            .map(|l| l.parse::<u64>().unwrap_or(100))
            .unwrap_or(100),
    }
}

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

fn create_list_item(entry: HistoryEntry) -> ListItem {
    ListItem {
        id: entry.id.clone(),
        icon: "iconoir-tools".to_string(),
        title: format!("{} (id: {})", entry.name, entry.id),
        sub_title: Some(format!(
            "user: {} | start_date: {} | end_date: {}",
            entry.user,
            entry.start_date_time.unwrap_or_default(),
            entry.end_date_time.unwrap_or("-".to_string())
        )),
        stat: entry.state,
    }
}

#[component]
pub fn HistoryList(
    #[prop(into)] state: Signal<Option<String>>,
    #[prop(into)] limit: Signal<Option<String>>,
    #[prop(into)] pipeline: Signal<Option<String>>,
    #[prop(into)] refresh: Signal<()>,
) -> impl IntoView {
    let (data, set_data) = create_signal(vec![]);

    let hist_res = create_resource(
        move || (state, limit, pipeline, set_data),
        |(state, limit, pipeline, set_data)| async move {
            let params = get_params(
                state.get_untracked(),
                limit.get_untracked(),
                pipeline.get_untracked(),
            );

            let data = get_hist(&params)
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .unwrap_or_default();

            // set_data.set(data.into_iter().map(create_list_item).collect());
            set_data.set(data);
        },
    );

    let _ = watch(
        move || refresh.get(),
        move |_, _, _| hist_res.refetch(),
        false,
    );

    let (headers, _) = create_signal(vec![
        "Id".to_string(),
        "Name".to_string(),
        "User".to_string(),
        "Start Date".to_string(),
        "End Date".to_string(),
        "State".to_string(),
    ]);

    let rows = move || {
        data.get()
            .into_iter()
            .map(|item| TableRow {
                columns: vec![
                    item.id,
                    item.name,
                    item.user,
                    item.start_date_time.unwrap_or_default(),
                    item.end_date_time.unwrap_or_default(),
                    item.state,
                ],
            })
            .collect()
    };

    view! {
        <div class="flex flex-col">
            // <div>
            //     {move || match hist_res.get() {
            //         None => view! { <div class="text-xl">Loading...</div> }.into_view(),
            //         Some(_) => view! { <List items=data /> }.into_view(),
            //     }}
            // </div>
            <Table headers=headers.into() rows=Signal::derive(rows) />
        </div>
    }
}
