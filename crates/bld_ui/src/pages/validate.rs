use crate::{api, error::ErrorCard};
use leptos::*;
use leptos_router::*;

#[component]
pub fn Validate() -> impl IntoView {
    let params = use_query_map();

    let resource = create_resource(
        move || params.get(),
        |params| async move {
            api::auth_validate(params.to_query_string())
                .await
                .map(|_| {
                    let nav = use_navigate();
                    nav("/", NavigateOptions::default());
                })
                .map_err(|e| e.to_string())
        },
    );

    view! {
        <Show when=move || matches!(resource.get(), Some(Err(_))) fallback=|| view! {}>
            <ErrorCard error=move || resource.get().unwrap().unwrap_err()/>
        </Show>
    }
}
