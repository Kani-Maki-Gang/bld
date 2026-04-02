use crate::{api, components::button::Button};
use leptos::*;
use leptos_dom::logging;

#[component]
pub fn Login() -> impl IntoView {
    view! {
        <div class="w-full flex items-center justify-center min-h-screen bg-zinc-950">
            <div class="flex flex-col items-center gap-8 w-full max-w-sm px-6">
                <div class="flex flex-col items-center gap-3">
                    <img class="size-24" src="logo_no_bg.png" />
                    <div class="text-lg font-semibold text-white tracking-tight">"bld"</div>
                </div>
                <div class="w-full bg-zinc-900 border border-zinc-800 rounded-2xl p-8 flex flex-col gap-6 shadow-2xl shadow-black/40">
                    <div class="flex flex-col gap-1">
                        <div class="text-xl font-semibold text-white">"Sign in"</div>
                        <div class="text-sm text-zinc-500">
                            "Authenticate via your OIDC provider to continue"
                        </div>
                    </div>
                    <Button on:click=move |_| {
                        if let Err(e) = api::auth_start() {
                            logging::console_error(&e.to_string());
                        }
                    }>"Continue with OIDC"</Button>
                </div>
                <div class="text-xs text-zinc-600">"Simple and blazingly fast CI/CD"</div>
            </div>
        </div>
    }
}
