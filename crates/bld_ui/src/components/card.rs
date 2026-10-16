use leptos::*;

#[component]
pub fn Card(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!("bg-surface border border-line rounded-[14px] {class}");
    view! { <div class=class>{children()}</div> }
}

#[component]
pub fn CardHeader(
    #[prop(into)] title: MaybeSignal<String>,
    #[prop(into, optional)] subtitle: MaybeSignal<String>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let (title, subtitle) = (
        Signal::derive(move || title.get()),
        Signal::derive(move || subtitle.get()),
    );
    view! {
        <div class="px-5 py-4 border-b border-line flex items-start justify-between gap-4 flex-wrap">
            <div class="min-w-0">
                <div class="text-[15px] font-semibold tracking-[-.01em]">{move || title.get()}</div>
                <Show when=move || !subtitle.get().is_empty() fallback=|| view! {}>
                    <div class="text-ink-muted text-[12.5px] mt-[3px]">
                        {move || subtitle.get()}
                    </div>
                </Show>
            </div>
            {children.map(|c| c())}
        </div>
    }
}

#[component]
pub fn Eyebrow(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!("font-mono text-[11px] tracking-[.08em] uppercase text-ink-muted {class}");
    view! { <span class=class>{children()}</span> }
}
