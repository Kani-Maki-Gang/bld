mod cron;
mod dashboard;
mod history;
mod monit;
mod pipelines;

pub use cron::*;
pub use dashboard::*;
pub use history::*;
pub use monit::*;
pub use pipelines::*;

use crate::{
    api,
    components::{
        button::Button,
        colors::Colors,
        topbar::Topbar,
    },
};
use leptos::{leptos_dom::logging, *};
use leptos_router::*;

#[component]
pub fn Home() -> impl IntoView {
    let auth_resource = create_resource(
        || (),
        |_| async move { api::check_auth_available().await.map_err(|e| e.to_string()) },
    );
    view! {
        <Show
            when=move || !auth_resource.loading().get()
            fallback=move || {
                view! {
                    <div class="flex items-center justify-center w-full h-full text-sm text-zinc-500">
                        "Loading..."
                    </div>
                }
            }
        >
            <div class="flex flex-col size-full">
                <Topbar>
                    <Button
                        color=Colors::Zinc
                        class="w-20"
                        on:click=move |_| {
                            if let Err(e) = api::remove_auth_tokens() {
                                logging::console_error(&e.to_string());
                            }
                            let nav = use_navigate();
                            nav("/login", NavigateOptions::default());
                        }
                    >
                        "Logout"
                    </Button>
                </Topbar>
                <main class="grow overflow-auto">
                    <Outlet/>
                </main>
            </div>
        </Show>
    }
}
