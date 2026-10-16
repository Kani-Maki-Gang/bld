use crate::{api, components::page::LogoMark};
use leptos::*;

#[component]
pub fn Login() -> impl IntoView {
    let reachable = create_resource(
        || (),
        |_| async move { api::check_auth_available().await.is_ok() },
    );
    let status = move || match reachable.get() {
        None => ("bg-ink-ghost", "checking server…"),
        Some(true) => ("bg-ok", "server reachable"),
        Some(false) => ("bg-bad", "sign-in unavailable"),
    };

    view! {
        <div class="min-h-screen grid place-items-center px-5 py-10 bg-[radial-gradient(120%_90%_at_50%_0%,#15161f_0%,#0d0e12_62%)]">
            <div class="w-full max-w-[380px] flex flex-col items-center gap-[26px]">
                <div class="flex flex-col items-center gap-3">
                    <LogoMark large=true />
                    <div class="font-mono text-[19px] font-semibold tracking-[-.02em]">"bld"</div>
                </div>
                <div class="w-full bg-surface border border-line rounded-[14px] px-6 pt-[26px] pb-6">
                    <div class="text-[17px] font-semibold tracking-[-.01em]">"Sign in"</div>
                    <div class="text-ink-muted text-[13px] mt-1.5 leading-normal">
                        "Authenticate via your OIDC provider to continue."
                    </div>
                    <button
                        class="mt-[22px] w-full h-10 rounded-[9px] bg-accent hover:bg-accent-hover text-white text-[13.5px] font-semibold tracking-[.01em] transition-colors"
                        on:click=move |_| {
                            if let Err(e) = api::auth_start() {
                                logging::error!("{e}");
                            }
                        }
                    >
                        "Continue with OIDC"
                    </button>
                    <div class="mt-3.5 flex items-center gap-2 justify-center font-mono text-[11px] text-ink-faint">
                        <span class=move || {
                            format!("size-[5px] rounded-full {}", status().0)
                        }></span>
                        <span>{move || status().1}</span>
                    </div>
                </div>
                <div class="text-xs text-ink-faint">"Simple and blazingly fast CI/CD"</div>
            </div>
        </div>
    }
}
