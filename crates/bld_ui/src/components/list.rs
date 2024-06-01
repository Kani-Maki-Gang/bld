use leptos::*;

#[component]
pub fn List(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-y-4 pr-2">
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
) -> impl IntoView {
    view! {
        <div class="flex justify-items-stretch items-center">
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
