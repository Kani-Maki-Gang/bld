use crate::components::card::Card;
use leptos::*;

#[derive(Debug, Clone, Default)]
pub struct Info {
    pub icon: String,
    pub count: u64,
    pub title: String,
    pub footnote: String,
}

#[component]
pub fn KpiInfo(#[prop(into)] info: Signal<Info>) -> impl IntoView {
    view! {
        <Card>
            <div class="px-6 py-6 flex flex-col gap-4">
                <div class="flex items-start justify-between">
                    <div class="text-sm font-medium text-zinc-400">{move || info.get().title}</div>
                    <div class="text-2xl text-violet-500 opacity-80">
                        <i class=move || info.get().icon></i>
                    </div>
                </div>
                <div class="text-4xl font-bold text-white tracking-tight">
                    {move || info.get().count}
                </div>
                <div class="text-xs text-zinc-500">{move || info.get().footnote}</div>
            </div>
        </Card>
    }
}
