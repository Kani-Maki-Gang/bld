use leptos::*;

#[component]
pub fn List(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!("flex flex-col gap-y-4 pr-2 overflow-y-auto {class}");
    view! {
        <div class=class>
            {children()}
        </div>
    }
}

#[component]
pub fn ComplexListItem(
    #[prop(into)] icon: Signal<String>,
    #[prop(into)] title: Signal<String>,
    #[prop(into, optional)] sub_title: Signal<String>,
    #[prop(into, optional)] stat: Signal<String>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let class = format!("flex justify-items-stretch items-center {class}");
    view! {
        <div class=class>
            <div class="text-5xl text-indigo-500">
                <i class=icon.get() />
            </div>
            <div class="flex flex-col ml-4 grow">
                <div>
                    {title.get()}
                </div>
                <div class="text-sm text-gray-400">
                    {sub_title.get()}
                </div>
            </div>
            <div class="text-xl">
                {stat.get()}
            </div>
        </div>
    }
}
