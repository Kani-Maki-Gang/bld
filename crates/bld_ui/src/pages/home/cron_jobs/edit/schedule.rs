use crate::components::{card::Card, input::Input};
use leptos::*;

#[component]
pub fn CronJobsEditSchedule(
    #[prop(into)] schedule: RwSignal<String>
) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-4">
                <div class="text-2xl">
                    "Cron job schedule"
                </div>
                <div class="text-gray-400">
                    "The schedule for the cron job provided in the cron expression format ("
                    <a class="text-blue-400 underline" target="_blank" href="https://en.wikipedia.org/wiki/Cron#Cron_expression">"Learn more"</a>
                    ")."
                </div>
                <div class="grid grid-cols-3 items-center">
                    <div>"Schedule"</div>
                    <div class="col-span-2">
                        <Input value=schedule />
                    </div>
                </div>
            </div>
        </Card>
    }
}
