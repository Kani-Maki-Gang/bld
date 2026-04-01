use crate::components::{card::Card, input::Input, link::Link};
use leptos::*;

#[component]
pub fn CronJobsEditSchedule(#[prop(into)] schedule: RwSignal<String>) -> impl IntoView {
    view! {
        <Card>
            <div class="flex flex-col px-6 py-5 gap-4">
                <div>
                    <div class="text-base font-semibold text-white">"Schedule"</div>
                    <div class="text-xs text-zinc-500 mt-0.5">
                        "Cron expression format — "
                        <Link href="https://en.wikipedia.org/wiki/Cron#Cron_expression".to_string()>
                            "learn more"
                        </Link>
                    </div>
                </div>
                <div class="grid grid-cols-3 items-center gap-4">
                    <div class="text-sm text-zinc-400">"Expression"</div>
                    <div class="col-span-2">
                        <Input value=schedule/>
                    </div>
                </div>
            </div>
        </Card>
    }
}
