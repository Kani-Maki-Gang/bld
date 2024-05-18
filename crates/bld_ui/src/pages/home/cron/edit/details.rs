use crate::components::{badge::Badge, button::Button, card::Card};
use bld_models::dtos::CronJobResponse;
use leptos::*;

#[component]
pub fn CronJobsEditDetails<F: Fn() -> () + 'static>(
    #[prop(into)] job: Signal<CronJobResponse>,
    save: F,
) -> impl IntoView {
    view! {
        <Card>
            <div class="flex px-8 py-12 items-start">
                <div class="grow flex flex-col gap-y-2">
                    <div class="text-2xl">
                        {job.get().pipeline}
                    </div>
                    <div class="flex gap-x-4">
                        <Show when=move || job.get().is_default fallback=|| view!{}>
                            <Badge>"Default"</Badge>
                        </Show>
                        <Show when=move || !job.get().date_created.is_empty() fallback=|| view!{}>
                            <Badge>"Created on: " {move || job.get().date_created}</Badge>
                        </Show>
                        <Show when=move || job.get().date_updated.is_some() fallback=|| view!{}>
                            <Badge>"Updated on: " {move || job.get().date_updated}</Badge>
                        </Show>
                    </div>
                </div>
                <div class="min-w-40">
                    <Button on:click=move |_| save()>"Save"</Button>
                </div>
            </div>
        </Card>
    }
}
