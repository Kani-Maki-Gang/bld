mod cron;
mod dashboard;
mod history;
mod monit;
mod pipelines;
mod shared;

pub use cron::*;
pub use dashboard::*;
pub use history::*;
pub use monit::*;
pub use pipelines::*;

use crate::{api, components::topbar::Topbar};
use leptos::*;
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
                    <div class="min-h-screen grid place-items-center text-[12.5px] text-ink-faint">
                        "Loading…"
                    </div>
                }
            }
        >
            <div class="min-h-screen flex flex-col">
                <Topbar />
                <main class="w-full max-w-[1480px] mx-auto px-[22px] pt-[26px] pb-10 flex-1 flex flex-col">
                    <Outlet />
                </main>
            </div>
        </Show>
    }
}
