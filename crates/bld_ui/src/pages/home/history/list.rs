use anyhow::Result;
use leptos::{leptos_dom::logging, *};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::components::{
    button::Button,
    list::{List, ListItem}
};

#[derive(Serialize, Deserialize, Debug)]
pub struct HistQueryParams {
    pub state: Option<String>,
    pub name: Option<String>,
    pub limit: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub name: String,
    pub id: String,
    pub user: String,
    pub state: String,
    pub start_date_time: Option<String>,
    pub end_date_time: Option<String>,
}

async fn get_hist() -> Result<Vec<HistoryEntry>> {
    let params = HistQueryParams {
        state: None,
        name: None,
        limit: 100,
    };

    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/hist")
        .query(&params)
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
pub fn HistoryList() -> impl IntoView {
    let (hist, set_hist) = create_signal(vec![]);

    let hist_res = create_resource(
        move || set_hist,
        |value| async move {
            let data = get_hist()
                .await
                .map_err(|e| logging::console_error(e.to_string().as_str()))
                .unwrap_or_default();

            value.set(
                data.into_iter()
                    .map(|h| ListItem {
                        id: h.id.clone(),
                        icon: "iconoir-tools".to_string(),
                        title: format!("{} (id: {})", h.name, h.id),
                        sub_title: Some(format!(
                            "user: {} | start_date: {} | end_date: {}",
                            h.user,
                            h.start_date_time.unwrap_or_default(),
                            h.end_date_time.unwrap_or("-".to_string())
                        )),
                        stat: h.state,
                    })
                    .collect(),
            );
        },
    );

    view! {
        <div class="flex flex-col">
            <Button on:click=move |_| hist_res.refetch()>
                "Refresh"
            </Button>
            <div>
                {move || match hist_res.get() {
                    None => view! { <div class="text-xl">Loading...</div> }.into_view(),
                    Some(_) => view! { <List items=hist /> }.into_view(),
                }}
            </div>
        </div>
    }
}
