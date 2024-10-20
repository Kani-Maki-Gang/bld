mod delete;
mod edit;
mod filters;
mod helpers;
mod insert;
mod new;
mod table;
mod update;

use crate::{components::card::Card, context::RefreshCronJobs};
use bld_models::dtos::JobFiltersParams;
use filters::CronJobsFilters;
use leptos::*;

pub use insert::CronJobInsert;
pub use table::CronJobsTable;
pub use update::CronJobUpdate;

fn get_params(limit: String, pipeline: String) -> Option<JobFiltersParams> {
    Some(JobFiltersParams::new(
        None,
        if pipeline.is_empty() {
            None
        } else {
            Some(pipeline)
        },
        None,
        None,
        limit.parse::<u64>().map(|x| Some(x)).unwrap_or_default(),
    ))
}

#[component]
pub fn CronJobs() -> impl IntoView {
    let pipeline = create_rw_signal(String::new());
    let limit = create_rw_signal("100".to_string());
    let params = move || get_params(limit.get(), pipeline.get());

    provide_context(RefreshCronJobs(create_rw_signal(())));

    view! {
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12">
                <div class="grid grid-cols-4">
                    <div class="grow flex flex-col">
                        <div class="text-2xl">"Cron jobs"</div>
                        <div class="text-gray-400 mb-8">
                            "A list of cron jobs for the current pipelines on the server"
                        </div>
                    </div>
                    <div class="col-span-3">
                        <CronJobsFilters limit=limit pipeline=pipeline/>
                    </div>
                </div>
                <CronJobsTable params=params/>
            </div>
        </Card>
    }
}
