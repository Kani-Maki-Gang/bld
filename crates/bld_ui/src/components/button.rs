use leptos::*;

#[component]
pub fn Button(children: Children) -> impl IntoView {
    view! {
        <button class="flex-non text-white rounded h-8 w-full bg-indigo-600 hover:bg-indigo-700">
            {children()}
        </button>
    }
}
