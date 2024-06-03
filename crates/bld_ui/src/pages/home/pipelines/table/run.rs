use crate::components::{colors::Colors, link::LinkIconButton};
use leptos::*;

#[component]
pub fn PipelineTableRunButton(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] name: Signal<String>,
) -> impl IntoView {
    view! {
        <LinkIconButton
            icon="iconoir-play"
            color=Colors::Green
            href=move || format!("/pipelines/run?id={}&name={}", id.get(), name.get()) />
    }
}
