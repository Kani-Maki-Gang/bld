use crate::components::badge::Badge;
use leptos::*;

#[component]
pub fn UserPill(#[prop(into)] name: Signal<String>) -> impl IntoView {
    view! {
        <Show when=move || !name.get().is_empty() fallback=|| view! {}>
            <Badge class="bg-zinc-800 border-zinc-700 text-zinc-300">
                <div class="flex items-center gap-1.5">
                    <i class="iconoir-user text-zinc-400 text-xs"></i>
                    <span>{name}</span>
                </div>
            </Badge>
        </Show>
    }
}
