mod cron_jobs;
mod dashboard;
mod history;
mod pipelines;

pub use cron_jobs::*;
pub use dashboard::*;
pub use history::*;
pub use pipelines::*;

use leptos::*;
use leptos_router::Outlet;
use crate::components::sidebar::{Sidebar, SidebarItem};

#[component]
pub fn Home() -> impl IntoView {
    let nav_items = vec![
        SidebarItem {
            icon: "iconoir-presentation".to_string(),
            text: "Dashboard".to_string(),
            url: "/".to_string(),
            ..Default::default()
        },
        SidebarItem {
            icon: "iconoir-book".to_string(),
            text: "History".to_string(),
            url: "/history".to_string(),
            ..Default::default()
        },
        SidebarItem {
            icon: "iconoir-wrench".to_string(),
            text: "Pipelines".to_string(),
            url: "/pipelines".to_string(),
            ..Default::default()
        },
        SidebarItem {
            icon: "iconoir-clock-rotate-right".to_string(),
            text: "Cron jobs".to_string(),
            url: "/cron".to_string(),
            ..Default::default()
        }
    ];

    view! {
        <div class="size-full flex">
            <div class="grow-0 flex self-stretch">
                <Sidebar items=nav_items />
            </div>
            <div class="m-12 grow overflow-auto">
                <Outlet />
            </div>
        </div>
    }
}
