use leptos::*;

#[component]
pub fn Button(children: Children) -> impl IntoView {
    view! {
        <button class="flex-non text-white rounded min-h-8 w-full bg-indigo-600 hover:bg-indigo-700 p-2">
            {children()}
        </button>
    }
}
