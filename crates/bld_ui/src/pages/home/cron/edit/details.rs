use crate::components::{badge::Badge, button::Button, card::Card, colors::Colors};
use bld_models::dtos::CronJobResponse;
use leptos::*;

#[component]
pub fn CronJobsEditDetails<F: Fn() -> () + 'static>(
    #[prop(into)] job: Signal<CronJobResponse>,
    save: F,
    delete: Option<WriteSignal<bool>>,
) -> impl IntoView {
    view! {
        <Card>
            <div class="flex px-6 py-5 items-center gap-3">
                <div class="grow flex flex-col gap-y-1.5">
                    <div class="text-base font-semibold text-white">{move || job.get().pipeline}</div>
                    <div class="flex gap-x-4">
                        <Show when=move || job.get().is_default fallback=|| view! {}>
                            <Badge>"Default"</Badge>
                        </Show>
                        <Show when=move || !job.get().date_created.is_empty() fallback=|| view! {}>
                            <Badge>"Created on: " {move || job.get().date_created}</Badge>
                        </Show>
                        <Show when=move || job.get().date_updated.is_some() fallback=|| view! {}>
                            <Badge>"Updated on: " {move || job.get().date_updated}</Badge>
                        </Show>
                    </div>
                </div>
                <div class="w-28">
                    <Button on:click=move |_| save()>"Save"</Button>
                </div>
                <Show when=move || delete.is_some() fallback=|| view! {}>
                    <div class="w-28">
                        <Button color=Colors::Red on:click=move |_| {
                            if let Some(delete) = delete {
                                delete.set(true);
                            }
                        }>"Delete"</Button>
                    </div>
                </Show>
            </div>
        </Card>
    }
}
