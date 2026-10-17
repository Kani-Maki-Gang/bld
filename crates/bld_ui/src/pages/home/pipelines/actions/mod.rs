mod copy;
mod delete;
mod r#move;

pub use copy::*;
pub use delete::*;
pub use r#move::*;

use crate::components::input::Input;
use leptos::*;

#[component]
fn DialogField(
    #[prop(into)] label: String,
    #[prop()] value: RwSignal<String>,
    #[prop(optional)] disabled: bool,
    #[prop(into, optional)] placeholder: String,
) -> impl IntoView {
    view! {
        <label class="mt-3.5 block">
            <div class="text-[11.5px] text-ink-muted">{label}</div>
            <div class="mt-1.5">
                <Input value=value disabled=disabled mono=true placeholder=placeholder />
            </div>
        </label>
    }
}
