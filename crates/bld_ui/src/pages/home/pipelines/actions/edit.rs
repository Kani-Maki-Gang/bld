use crate::components::{colors::Colors, link::LinkIconButton};
use leptos::*;

#[component]
pub fn PipelineEditButton(
    #[prop(into)] id: Signal<String>,
    #[prop(into)] name: Signal<String>,
) -> impl IntoView {
    let href = move || with!(|id, name| format!("/pipelines/info?id={}&name={}", id, name));
    view! { <LinkIconButton href=href icon="iconoir-edit" color=Colors::Zinc/> }
}
