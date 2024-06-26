use crate::components::card::Card;
use leptos::*;
use leptos_router::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="w-full flex justify-center self-center">
            <div class="w-96">
                <Card>
                    <div class="flex flex-col p-8">
                        <div class="flex items-center space-x-4 mb-12">
                            <div class="text-6xl text-red-400">
                                <i class="iconoir-info-circle"></i>
                            </div>
                            <div class="text-2xl">"Page not found!"</div>
                        </div>
                        <div class="flex justify-center text-xl text-blue-400 underline">
                            <A href="/">Go home</A>
                        </div>
                    </div>
                </Card>
            </div>
        </div>
    }
}
