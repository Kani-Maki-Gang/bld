use leptos::*;

#[component]
pub fn Error(#[prop(into)] error: Signal<String>) -> impl IntoView {
    view! {
        <div class="text-red-500 text-center text-8xl">
            <i class="iconoir-cloud-xmark"></i>
        </div>
        <div class="text-center">"Failed to fetch data due to: " {move || error.get()}</div>
        <div class="text-center text-gray-400">"Please try again later"</div>
    }
}
