use crate::components::card::Card;
use leptos::*;

#[derive(Debug, Clone, Default)]
pub struct Info {
    pub icon: String,
    pub count: i32,
    pub title: String,
    pub footnote: String,
}

#[component]
pub fn KpiInfo(#[prop()] info: ReadSignal<Info>) -> impl IntoView {
    view! {
        <Card>
            <div class="px-8 py-12 flex flex-col">
                <div class="text-xl grid grid-cols-2 items-center">
                    <div class="grow">{move || info.get().title}</div>
                    <div class="justify-self-end text-5xl text-indigo-500">
                        <i class=move || info.get().icon></i>
                    </div>
                </div>
                <div class="my-4 text-6xl">{move || info.get().count}</div>
                <div class="text-gray-400">{move || info.get().footnote}</div>
            </div>
        </Card>
    }
}
