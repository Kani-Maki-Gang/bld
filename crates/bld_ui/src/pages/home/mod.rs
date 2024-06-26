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

use crate::components::sidebar::{Sidebar, SidebarItem};
use leptos::*;
use leptos_router::Outlet;

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="size-full flex">
            <div class="grow-0 flex self-stretch">
                <Sidebar>
                    <SidebarItem icon="iconoir-presentation" text="Dashboard" url="/"/>
                    <SidebarItem icon="iconoir-book" text="History" url="/history"/>
                    <SidebarItem icon="iconoir-wrench" text="Pipelines" url="/pipelines"/>
                    <SidebarItem icon="iconoir-clock-rotate-right" text="Cron jobs" url="/cron"/>
                </Sidebar>
            </div>
            <div class="grow overflow-auto p-4">
                <Outlet/>
            </div>
        </div>
    }
}
