use crate::components::button::Button;
use leptos::*;
use leptos_router::A;

#[derive(Debug, Clone, Default)]
pub struct SidebarItem {
    pub icon: String,
    pub text: String,
    pub url: String,
}

#[component]
pub fn SidebarTop() -> impl IntoView {
    view! {
        <div class="bg-slate-800 text-center grid justify-items-center">
            <img class="p-4 size-48" src="logo.png" />
        </div>
    }
}

#[component]
pub fn SidebarItemInstance(#[prop()] item: SidebarItem) -> impl IntoView {
    view! {
        <A class="py-4 px-8 hover:bg-slate-600 hover:cursor-pointer flex items-center" href=item.url>
            <div class="text-2xl text-indigo-500">
                <i class=item.icon />
            </div>
            <div class="ml-4">{item.text}</div>
        </A>
    }
}

#[component]
pub fn SidebarContent(#[prop()] items: Vec<SidebarItem>) -> impl IntoView {
    view! {
        <div class="flex flex-col divide-y divide-slate-600">
            {items
                .into_iter()
                .map(|i| view!{ <SidebarItemInstance item=i /> })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn SidebarBottom() -> impl IntoView {
    view! {
        <div class="flex-none p-8 text-center">
            <div class="mb-4">
                "Star the project on "
                <a class="text-blue-400 underline" target="_blank" href="https://github.com/Kani-Maki-Gang/bld">"GitHub"</a>
                " and checkout our "
                <a class="text-blue-400 underline" target="_blank" href="https://kani-maki-gang.github.io/bld-book/">"book"</a>
            </div>
            <Button>"Logout"</Button>
        </div>
    }
}

#[component]
pub fn Sidebar(#[prop()] items: Vec<SidebarItem>) -> impl IntoView {
    view! {
        <div class="bg-slate-700 w-64 shadow-md flex flex-col divide-y divide-slate-600">
            <SidebarTop />
            <div class="grow">
                <SidebarContent items=items />
            </div>
            <SidebarBottom />
        </div>
    }
}
