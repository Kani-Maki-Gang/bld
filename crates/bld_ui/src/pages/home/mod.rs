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

fn get_nav_items() -> Vec<SidebarItem> {
    vec![
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
        },
    ]
}

#[component]
pub fn Home() -> impl IntoView {
    let nav_items = get_nav_items();

    view! {
        <div class="size-full flex">
            <div class="grow-0 flex self-stretch">
                <Sidebar items=nav_items />
            </div>
            <div class="grow overflow-auto">
                <div class="m-12">
                    <Outlet />
                </div>
            </div>
        </div>
    }
}
