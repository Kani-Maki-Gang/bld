use leptos::*;
use crate::components::sidebar::{Sidebar, SidebarItem};

#[component]
pub fn Home() -> impl IntoView {
    let nav_items = vec![
        SidebarItem {
            icon: "iconoir-presentation".to_string(),
            text: "Dashboard".to_string(),
            ..Default::default()
        },
        SidebarItem {
            icon: "iconoir-book".to_string(),
            text: "History".to_string(),
            ..Default::default()
        },
        SidebarItem {
            icon: "iconoir-wrench".to_string(),
            text: "Pipelines".to_string(),
            ..Default::default()
        },
        SidebarItem {
            icon: "iconoir-clock-rotate-right".to_string(),
            text: "Cron jobs".to_string(),
            ..Default::default()
        }
    ];

    view! {
        <div class="h-screen flex">
            <Sidebar items=nav_items />
        </div>
    }
}
