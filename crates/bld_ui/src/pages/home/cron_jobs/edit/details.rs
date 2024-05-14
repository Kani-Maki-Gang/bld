use crate::components::{badge::Badge, card::Card};
use bld_models::dtos::CronJobResponse;
use leptos::*;

#[component]
pub fn CronJobsEditDetails(#[prop(into)] job: Signal<CronJobResponse>) -> impl IntoView {
    view! {
        <Card>
            <div class="flex px-8 py-12">
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
            </div>
        </Card>
    }
}
