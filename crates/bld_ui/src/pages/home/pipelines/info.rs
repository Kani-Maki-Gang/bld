use anyhow::Result;
use bld_models::dtos::PipelineInfoQueryParams;
use leptos::*;
use leptos_router::*;
use reqwest::Client;
use serde_json::Value;

async fn get_pipeline(id: String) -> Result<Value> {
    let params = PipelineInfoQueryParams::Id { id };

    let res = Client::builder()
        .build()?
        .get("http://localhost:6080/v1/print")
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        Ok(serde_json::from_str(&body)?)
    } else {
        Ok(Value::Null)
    }
}

#[component]
pub fn PipelineInfo() -> impl IntoView {
    let params = use_params_map();

    let pipeline_id = move || {
        params.with(|p| p.get("id").cloned())
    };

    let (_data, set_data) = create_signal(Value::Null);

    let _ = create_resource(
        move || (pipeline_id(), set_data),
        |(id, set_data)| async move {
            let Some(id) = id else {
                return;
            };

            let value = get_pipeline(id)
                .await
                .unwrap_or_else(|_| Value::Null);

            if value == Value::Null {
                set_data.set(value);
            }
        }
    );

    view! {
        "Hello from the pipeline info page for pipeline with id: " {pipeline_id}
    }
}
