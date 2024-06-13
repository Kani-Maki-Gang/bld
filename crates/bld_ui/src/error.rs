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
