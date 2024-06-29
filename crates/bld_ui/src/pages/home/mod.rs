mod cron;
mod dashboard;
mod history;
mod monit;
mod pipelines;

pub use cron::*;
pub use dashboard::*;
pub use history::*;
use leptos_dom::logging;
use leptos_use::{storage::use_local_storage, utils::FromToStringCodec};
pub use monit::*;
pub use pipelines::*;

use crate::{
    api,
    components::sidebar::{Sidebar, SidebarItem},
};
use leptos::*;
use leptos_router::Outlet;

#[component]
pub fn Home() -> impl IntoView {
    let auth_resource = create_resource(
        || (),
        |_| async move {
            let res = api::auth_available().await;
            let (_, write, _) = use_local_storage::<bool, FromToStringCodec>("auth_available");
            match res {
                Ok(_) => {
                    write.set(true);
                }
                Err(e) => {
                    let error = format!("Error checking auth availability: {e}");
                    logging::console_log(&error);
                    write.set(false);
                }
            }
        },
    );
    view! {
        <Show
            when=move || !auth_resource.loading().get()
            fallback=move || view! { <div class="text-xl text-gray-400">"Loading..."</div> }
        >
            <div class="size-full flex">
                <div class="grow-0 flex self-stretch">
                    <Sidebar>
                        <SidebarItem icon="iconoir-presentation" text="Dashboard" url="/"/>
                        <SidebarItem icon="iconoir-book" text="History" url="/history"/>
                        <SidebarItem icon="iconoir-wrench" text="Pipelines" url="/pipelines"/>
                        <SidebarItem
                            icon="iconoir-clock-rotate-right"
                            text="Cron jobs"
                            url="/cron"
                        />
                    </Sidebar>
                </div>
                <div class="grow overflow-auto p-4">
                    <Outlet/>
                </div>
            </div>
        </Show>
    }
}
