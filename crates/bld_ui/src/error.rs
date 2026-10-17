use crate::{
    components::{
        button::{Button, ButtonVariant},
        card::Card,
    },
    context::use_app_dialog,
};
use leptos::*;

#[component]
pub fn ErrorBanner(#[prop(into)] error: MaybeSignal<String>) -> impl IntoView {
    view! {
        <div class="flex items-start gap-2.5 px-4 py-3 rounded-[10px] border border-bad-line bg-bad/[.07] text-[12.5px] text-bad-soft">
            <span class="mt-[5px] size-1.5 rounded-full bg-bad shrink-0"></span>
            <div class="min-w-0 break-words">"Failed to fetch data: " {move || error.get()}</div>
        </div>
    }
}

#[component]
pub fn ErrorCard(#[prop(into)] error: MaybeSignal<String>) -> impl IntoView {
    view! {
        <Card class="px-6 py-12 flex flex-col items-center gap-2 text-center">
            <span class="size-2 rounded-full bg-bad"></span>
            <div class="text-sm font-medium mt-1">"Could not load this page"</div>
            <div class="text-[12.5px] text-ink-muted max-w-[420px] break-words">
                {move || error.get()}
            </div>
        </Card>
    }
}

#[component]
pub fn DialogCard(
    #[prop(into)] title: String,
    #[prop(into, optional)] subtitle: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="w-[min(420px,calc(100vw-32px))] bg-surface border border-line-edge rounded-[14px] px-[22px] pt-[22px] pb-5 shadow-[0_24px_70px_rgba(0,0,0,.6)] flex flex-col">
            <div class="text-base font-semibold tracking-[-.01em]">{title}</div>
            <Show when={
                let empty = subtitle.is_empty();
                move || !empty
            }>
                <div class="text-ink-muted text-[12.5px] mt-[5px]">{subtitle.clone()}</div>
            </Show>
            {children()}
        </div>
    }
}

#[component]
pub fn ErrorDialog(#[prop(into)] error: String) -> impl IntoView {
    let dialog = use_app_dialog();
    view! {
        <DialogCard title="Something went wrong">
            <div class="mt-3 text-[12.5px] text-bad-soft break-words">{error}</div>
            <div class="mt-5 flex">
                <Button
                    variant=ButtonVariant::Ghost
                    size=crate::components::button::ButtonSize::Lg
                    class="flex-1"
                    on:click=move |_| dialog.close()
                >
                    "Close"
                </Button>
            </div>
        </DialogCard>
    }
}
