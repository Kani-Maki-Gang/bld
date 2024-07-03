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
        sidebar::{Sidebar, SidebarBottom, SidebarItem, SidebarTop},
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
            fallback=move || view! { <div class="text-xl text-gray-400">"Loading..."</div> }
        >
            <div class="size-full flex">
                <div class="grow-0 flex self-stretch">
                    <Sidebar>
                        <SidebarTop/>
                        <div class="grow flex flex-col divide-y divide-slate-600">
                            <SidebarItem icon="iconoir-presentation" text="Dashboard" url="/"/>
                            <SidebarItem icon="iconoir-book" text="History" url="/history"/>
                            <SidebarItem icon="iconoir-wrench" text="Pipelines" url="/pipelines"/>
                            <SidebarItem
                                icon="iconoir-clock-rotate-right"
                                text="Cron jobs"
                                url="/cron"
                            />
                        </div>
                        <SidebarBottom>
                            <Button on:click=move |_| {
                                if let Err(e) = api::remove_auth_tokens() {
                                    logging::console_error(&e.to_string());
                                }
                                let nav = use_navigate();
                                nav("/login", NavigateOptions::default());
                            }>
                                "Logout"
                            </Button>
                        </SidebarBottom>
                    </Sidebar>
                </div>
                <div class="grow overflow-auto p-4">
                    <Outlet/>
                </div>
            </div>
        </Show>
    }
}
