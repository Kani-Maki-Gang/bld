use leptos::*;

#[component]
pub fn List(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!("flex flex-col gap-y-3 pr-2 overflow-y-auto {class}");
    view! { <div class=class>{children()}</div> }
}

#[component]
pub fn ComplexListItem(
    #[prop(into)] icon: Signal<String>,
    #[prop(into)] title: Signal<String>,
    #[prop(into, optional)] sub_title: Signal<String>,
    #[prop(into, optional)] stat: Signal<String>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let class = format!(
        "flex justify-items-stretch items-center px-3 py-2.5 rounded-lg hover:bg-zinc-800/50 transition-colors duration-100 {class}"
    );
    view! {
        <div class=class>
            <div class="text-3xl text-violet-500 w-10 shrink-0">
                <i class=icon.get()></i>
            </div>
            <div class="flex flex-col ml-3 grow min-w-0">
                <div class="text-sm font-medium text-zinc-100 truncate">{title.get()}</div>
                <div class="text-xs text-zinc-500">{sub_title.get()}</div>
            </div>
            <div class="text-sm font-medium text-zinc-300 shrink-0 ml-4">{stat.get()}</div>
        </div>
    }
}
