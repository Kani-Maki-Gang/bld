use crate::components::badge::Badge;
use leptos::*;

#[component]
pub fn UserPill(#[prop(into)] name: Signal<String>) -> impl IntoView {
    view! {
        <Show when=move || !name.get().is_empty() fallback=|| view! {}>
            <Badge class="bg-slate-800">
                <div class="flex items-center">
                    <div class="flex items-center justify-center">
                        <i class="iconoir-user text-white"></i>
                    </div>
                    <span class="ml-2">{name}</span>
                </div>
            </Badge>
        </Show>
    }
}
