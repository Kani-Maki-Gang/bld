use leptos::*;
use crate::components::sidebar::{Sidebar, SidebarItem};

#[component]
pub fn Home() -> impl IntoView {
    let nav_items = vec![
        SidebarItem {
            text: "Item 1".to_string(),
            ..Default::default()
        },
        SidebarItem {
            text: "Item 2".to_string(),
            ..Default::default()
        }
    ];

    view! {
        <div class="h-screen flex">
            <Sidebar items=nav_items />
        </div>
    }
}
