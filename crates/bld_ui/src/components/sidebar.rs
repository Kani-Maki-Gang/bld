use crate::components::button::Button;
use leptos::*;
use leptos_router::A;

#[component]
pub fn SidebarTop() -> impl IntoView {
    view! {
        <div class="bg-slate-800 grid justify-items-center">
            <img class="p-4 size-48" src="logo.png"/>
        </div>
    }
}

#[component]
pub fn SidebarItem(
    #[prop(into)] icon: String,
    #[prop(into)] text: String,
    #[prop(into)] url: String,
) -> impl IntoView {
    view! {
        <A class="py-4 px-8 hover:bg-slate-600 hover:cursor-pointer flex items-center" href=url>
            <div class="text-2xl text-indigo-500">
                <i class=icon></i>
            </div>
            <div class="ml-4">{text}</div>
        </A>
    }
}

#[component]
pub fn SidebarBottom() -> impl IntoView {
    view! {
        <div class="flex-none p-8 text-center">
            <div class="mb-4">
                "Star the project on "
                <a
                    class="text-blue-400 underline"
                    target="_blank"
                    href="https://github.com/Kani-Maki-Gang/bld"
                >
                    "GitHub"
                </a> " and checkout our "
                <a
                    class="text-blue-400 underline"
                    target="_blank"
                    href="https://kani-maki-gang.github.io/bld-book/"
                >
                    "book"
                </a>
            </div>
            <Button>"Logout"</Button>
        </div>
    }
}

#[component]
pub fn Sidebar(children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-700 w-64 shadow-md flex flex-col divide-y divide-slate-600">
            <SidebarTop/>
            <div class="grow flex flex-col divide-y divide-slate-600">{children()}</div>
            <SidebarBottom/>
        </div>
    }
}
