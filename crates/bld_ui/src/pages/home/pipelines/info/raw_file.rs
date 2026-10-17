use super::PipelineFile;
use crate::{
    browser::copy_to_clipboard,
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        card::Card,
    },
};
use leptos::*;

#[component]
pub fn PipelineRawFile(#[prop()] file: PipelineFile) -> impl IntoView {
    let copied = create_rw_signal(false);
    let raw = file.raw.clone();
    view! {
        <Card class="mt-[18px] overflow-hidden flex-1 flex flex-col">
            <div class="px-[18px] py-[13px] border-b border-line flex items-center justify-between gap-3">
                <span class="font-mono text-[11.5px] text-ink-muted">{file.summary()}</span>
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::Xs
                    on:click=move |_| {
                        if let Err(e) = copy_to_clipboard(&raw) {
                            logging::error!("{e}");
                        } else {
                            copied.set(true);
                        }
                    }
                >
                    {move || if copied.get() { "Copied" } else { "Copy" }}
                </Button>
            </div>
            <pre class="m-0 px-5 py-[18px] font-mono text-[12.5px] leading-[1.72] text-ink-soft overflow-auto flex-1 basis-0 min-h-[560px] whitespace-pre">
                {file.raw}
            </pre>
        </Card>
    }
}
