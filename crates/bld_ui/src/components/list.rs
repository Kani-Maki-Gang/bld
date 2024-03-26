use leptos::*;

#[derive(Debug, Clone)]
pub struct ListItem {
    pub id: String,
    pub icon: String,
    pub title: String,
    pub sub_title: String,
    pub stat: String,
}

#[component]
pub fn List(#[prop()] items: ReadSignal<Vec<ListItem>>) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-y-4">
            <For each=move || items.get() key=|state| state.id.clone() let:child>
                <div class="flex justify-items-stretch items-center">
                    <div class="text-5xl text-indigo-400">
                        <i class={child.icon} />
                    </div>
                    <div class="flex flex-col ml-4 grow">
                        <div>
                            {child.title}
                        </div>
                        <div class="text-sm text-gray-400">
                            {child.sub_title}
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
