use crate::components::{badge::Badge, button::Button, card::Card};
use bld_models::dtos::CronJobResponse;
use leptos::*;

#[component]
pub fn CronJobsEditDetails<F: Fn() -> () + 'static>(
    #[prop(into)] job: Signal<CronJobResponse>,
    save: F
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
                        <Badge>"Created on: " {move || job.get().date_created}</Badge>
                        <Badge>"Updated on: " {move || job.get().date_updated}</Badge>
                    </div>
                </div>
                <div class="min-w-40">
                    <Button on:click=move |_| save()>"Save"</Button>
                </div>
            </div>
        </Card>
    }
}
