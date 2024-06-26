use crate::components::{colors::Colors, link::LinkIconButton};
use leptos::*;

#[component]
pub fn PipelineRunButton(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] name: Signal<String>,
) -> impl IntoView {
    view! {
        <LinkIconButton
            icon="iconoir-play"
            color=Colors::Zinc
            href=move || format!("/pipelines/run?id={}&name={}", id.get(), name.get())
        />
    }
}
