use leptos::*;

#[derive(Debug, Clone, Default)]
pub struct ListItem {
    pub id: String,
    pub icon: String,
    pub title: String,
    pub sub_title: Option<String>,
    pub content: Option<View>,
    pub stat: String,
}

#[component]
pub fn List(#[prop(into)] items: Signal<Vec<ListItem>>) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-y-4 pr-2">
            <For each=move || items.get() key=|state| state.id.clone() let:child>
                <div class="flex justify-items-stretch items-center">
                    <div class="text-5xl text-indigo-500">
                        <i class={child.icon} />
                    </div>
                    <div class="flex flex-col ml-4 grow">
                        <div>
                            {child.title}
                        </div>
                        <div class="text-sm text-gray-400">
                            {child.sub_title}
                        </div>
                        <div>
                            {child.content}
                        </div>
                    </div>
                    <div class="text-xl">
                        {child.stat}
                    </div>
                </div>
            </For>
        </div>
    }
}
