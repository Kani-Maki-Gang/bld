use leptos::*;

#[component]
pub fn Button(children: Children) -> impl IntoView {
    view! {
        <button class="h-[40px] flex-non text-white rounded-lg w-full bg-indigo-600 p-2 hover:bg-indigo-700 focus:bg-indigo-700 focus:outline-none">
            {children()}
        </button>
    }
}