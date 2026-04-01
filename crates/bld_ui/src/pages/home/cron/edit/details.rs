use crate::components::{badge::Badge, button::Button, colors::Colors};
use bld_models::dtos::CronJobResponse;
use leptos::*;

#[component]
pub fn CronJobsEditDetails<F: Fn() -> () + 'static>(
    #[prop(into)] job: Signal<CronJobResponse>,
    save: F,
    delete: Option<WriteSignal<bool>>,
) -> impl IntoView {
    view! {
        <div class="px-6 py-5 border-b border-zinc-800 flex items-center gap-3">
            <div class="grow">
                <div class="text-lg font-semibold text-white">{move || job.get().pipeline}</div>
                <div class="flex gap-2 mt-1 flex-wrap">
                    <Show when=move || job.get().is_default fallback=|| view! {}>
                        <Badge>"Default"</Badge>
                    </Show>
                    <Show when=move || !job.get().date_created.is_empty() fallback=|| view! {}>
                        <Badge>"Created " {move || job.get().date_created}</Badge>
                    </Show>
                    <Show when=move || job.get().date_updated.is_some() fallback=|| view! {}>
                        <Badge>"Updated " {move || job.get().date_updated}</Badge>
                    </Show>
                </div>
            </div>
            <div class="w-28 shrink-0">
                <Button on:click=move |_| save()>"Save"</Button>
            </div>
            <Show when=move || delete.is_some() fallback=|| view! {}>
                <div class="w-28 shrink-0">
                    <Button
                        color=Colors::Red
                        on:click=move |_| {
                            if let Some(delete) = delete {
                                delete.set(true);
                            }
                        }
                    >
                        "Delete"
                    </Button>
                </div>
            </Show>
        </div>
    }
}
