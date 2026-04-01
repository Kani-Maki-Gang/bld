mod delete;
mod edit;
mod filters;
mod helpers;
mod insert;
mod new;
mod table;
mod update;

use crate::context::RefreshCronJobs;
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
        <div class="flex flex-col min-h-full">
            <div class="px-6 py-5 border-b border-zinc-800 flex items-center gap-4">
                <div class="grow">
                    <div class="text-lg font-semibold text-white">"Cron jobs"</div>
                    <div class="text-xs text-zinc-500 mt-0.5">
                        "Scheduled pipeline runs on the server"
                    </div>
                </div>
            </div>
            <div class="px-6 py-3 border-b border-zinc-800/60">
                <CronJobsFilters limit=limit pipeline=pipeline />
            </div>
            <div class="px-6 py-5">
                <CronJobsTable params=params />
            </div>
        </div>
    }
}
