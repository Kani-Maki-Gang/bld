use super::new::CronJobsNewButton;
use crate::components::input::Input;
use leptos::*;

#[component]
pub fn CronJobsFilters(
    #[prop(into)] pipeline: RwSignal<String>,
    #[prop(into)] limit: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-3">
            <div class="col-span-2">
                <Input placeholder="Search..." value=pipeline/>
            </div>
            <div class="flex justify-end gap-4">
                <div class="min-w-[100px]">
                    <Input input_type="number" placeholder="Limit" value=limit/>
                </div>
                <CronJobsNewButton/>
            </div>
        </div>
    }
}
