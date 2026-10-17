use leptos::*;

#[component]
pub fn PageHeader(
    #[prop(into)] title: MaybeSignal<String>,
    #[prop(into, optional)] subtitle: MaybeSignal<String>,
    #[prop(optional)] back: Option<View>,
    #[prop(optional)] mono: bool,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let (title, subtitle) = (
        Signal::derive(move || title.get()),
        Signal::derive(move || subtitle.get()),
    );
    let title_class = if mono {
        "font-mono text-xl font-semibold tracking-[-.02em] break-all"
    } else {
        "text-[21px] font-semibold tracking-[-.02em]"
    };
    view! {
        <div class="flex items-end justify-between gap-5 flex-wrap">
            <div class="min-w-0">
                {back} <div class=title_class>{move || title.get()}</div>
                <Show when=move || !subtitle.get().is_empty() fallback=|| view! {}>
                    <div class="text-ink-muted text-[13px] mt-[3px]">{move || subtitle.get()}</div>
                </Show>
            </div>
            <div class="flex items-center gap-2 flex-wrap">{children.map(|c| c())}</div>
        </div>
    }
}

#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(into)] body: String,
    #[prop(optional)] visual: Option<View>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="px-[22px] py-12 flex flex-col items-center gap-2.5 text-center">
            {visual} <div class="text-sm font-medium mt-1">{title}</div>
            <div class="text-[12.5px] text-ink-muted max-w-[360px] leading-[1.55]">{body}</div>
            <div class="mt-1.5 flex gap-2 flex-wrap justify-center">{children.map(|c| c())}</div>
        </div>
    }
}

#[component]
pub fn LoadingState() -> impl IntoView {
    view! { <div class="px-5 py-10 text-center text-[12.5px] text-ink-faint">"Loading…"</div> }
}

#[component]
pub fn LogoMark(#[prop(optional)] large: bool) -> impl IntoView {
    let (outer, inner) = if large {
        (
            "size-[52px] rounded-[14px] shadow-[0_0_0_1px_rgba(108,92,245,.16),0_10px_30px_rgba(0,0,0,.5)]",
            "size-[34px]",
        )
    } else {
        ("size-[26px] rounded-lg", "size-[18px]")
    };
    view! {
        <div class=format!(
            "bg-surface border border-line-edge grid place-items-center shrink-0 {outer}",
        )>
            <img class=inner src="/logo_no_bg.png" alt="bld" />
        </div>
    }
}
