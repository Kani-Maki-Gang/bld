use crate::components::card::Eyebrow;
use leptos::{html::Div, *};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogStatus {
    Connecting,
    Following,
    Ended,
}

#[component]
pub fn MonitLogs(
    #[prop(into)] lines: Signal<Vec<String>>,
    #[prop(into)] status: Signal<LogStatus>,
) -> impl IntoView {
    let container = create_node_ref::<Div>();
    let stick = create_rw_signal(true);
    let last_top = store_value(0);

    create_effect(move |_| {
        lines.track();
        if stick.get_untracked() {
            request_animation_frame(move || {
                if let Some(el) = container.get_untracked() {
                    el.set_scroll_top(el.scroll_height());
                }
            });
        }
    });

    let status_view = move || match status.get() {
        LogStatus::Connecting => ("bg-ink-ghost", "connecting"),
        LogStatus::Following => ("bg-ok", "following"),
        LogStatus::Ended => ("bg-ink-faint", "ended"),
    };

    view! {
        <div class="mt-3.5 bg-well border border-line rounded-[14px] overflow-hidden flex-1 flex flex-col">
            <div class="px-4 py-[11px] border-b border-line-soft flex items-center justify-between gap-3">
                <Eyebrow>"stdout"</Eyebrow>
                <span class="flex items-center gap-[7px] text-[11.5px] text-ink-muted">
                    <span class=move || {
                        format!("size-[5px] rounded-full {}", status_view().0)
                    }></span>
                    {move || status_view().1}
                </span>
            </div>
            <Show
                when=move || !lines.with(|l| l.is_empty())
                fallback=move || {
                    view! {
                        <div class="px-[22px] py-10 flex flex-col items-center gap-2 text-center">
                            <div class="font-mono text-[12.5px] text-ink-muted">
                                "no output yet"
                                <span class="inline-block w-[7px] h-3.5 bg-ink-ghost ml-1 align-[-2px] animate-bld-caret"></span>
                            </div>
                            <div class="text-[12.5px] text-ink-muted max-w-[320px] leading-[1.55]">
                                {move || {
                                    if status.get() == LogStatus::Ended {
                                        "This run wrote no output."
                                    } else {
                                        "Lines stream in as the job writes them. The view follows the tail automatically."
                                    }
                                }}
                            </div>
                        </div>
                    }
                }
            >
                <div
                    node_ref=container
                    class="py-3.5 flex-1 basis-0 min-h-[640px] overflow-auto"
                    on:scroll=move |_| {
                        if let Some(el) = container.get_untracked() {
                            let top = el.scroll_top();
                            let distance = el.scroll_height() - top - el.client_height();
                            if distance < 80 {
                                stick.set(true);
                            } else if top < last_top.get_value() {
                                stick.set(false);
                            }
                            last_top.set_value(top);
                        }
                    }
                >
                    <For each=move || lines.get().into_iter().enumerate() key=|(i, _)| *i let:line>
                        <div class="flex gap-3.5 px-4 py-[1.5px] font-mono text-[12.5px] leading-[1.6] hover:bg-surface">
                            <span class="text-ink-ghost w-[52px] shrink-0 text-right select-none">
                                {line.0 + 1}
                            </span>
                            <span class="text-ink-soft whitespace-pre-wrap break-words min-w-0">
                                {line.1}
                            </span>
                        </div>
                    </For>
                    <Show when=move || status.get() == LogStatus::Following fallback=|| view! {}>
                        <div class="flex gap-3.5 px-4 py-[1.5px] font-mono text-[12.5px]">
                            <span class="w-[52px] shrink-0"></span>
                            <span class="w-[7px] h-[15px] bg-ink-muted animate-bld-caret"></span>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
