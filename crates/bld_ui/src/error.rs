use crate::components::{button::Button, card::Card};
use leptos::{html::Dialog, *};

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

#[component]
pub fn SmallError(#[prop(into)] error: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex items-center rounded-lg bg-red-500 p-2 gap-4">
            <div class="text-2xl">
                <i class="iconoir-cloud-xmark"></i>
            </div>
            <div class="flex flex-col">
                <div>"Failed to fetch data."</div>
                <div class="text-gray-200">{move || error.get()}</div>
            </div>
        </div>
    }
}

#[component]
pub fn ErrorCard(#[prop(into)] error: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center">
            <Card class="container px-8 py-12">
                <Error error=move || error.get()/>
            </Card>
        </div>
    }
}

#[component]
pub fn ErrorDialog(
    #[prop(into)] dialog: NodeRef<Dialog>,
    #[prop(into)] error: Signal<String>,
) -> impl IntoView {
    view! {
        <Card class="flex flex-col gap-4 px-8 py-12 h-[600px] w-[500px]">
            <div class="grow">
                <Error error=move || error.get()/>
            </div>
            <Button on:click=move |_| {
                let _ = dialog.get().map(|x| x.close());
            }>"Close"</Button>
        </Card>
    }
}
