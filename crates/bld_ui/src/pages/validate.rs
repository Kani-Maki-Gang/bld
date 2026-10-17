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
        <div class="min-h-screen grid place-items-center px-5">
            {move || match resource.get() {
                Some(Err(e)) => view! { <ErrorCard error=e /> }.into_view(),
                _ => {
                    view! { <div class="text-[12.5px] text-ink-faint">"Signing you in…"</div> }
                        .into_view()
                }
            }}
        </div>
    }
}
