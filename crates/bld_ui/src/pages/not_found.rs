use crate::components::{button::ButtonVariant, card::Card, link::LinkButton};
use leptos::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="min-h-screen grid place-items-center px-5">
            <Card class="w-full max-w-[380px] px-6 py-10 flex flex-col items-center gap-2.5 text-center">
                <div class="font-mono text-[11px] tracking-[.09em] uppercase text-ink-faint">
                    "404"
                </div>
                <div class="text-[17px] font-semibold">"Page not found"</div>
                <div class="text-[12.5px] text-ink-muted">
                    "The page you asked for does not exist."
                </div>
                <LinkButton href="/" variant=ButtonVariant::Secondary class="mt-2">
                    "Go to overview"
                </LinkButton>
            </Card>
        </div>
    }
}
