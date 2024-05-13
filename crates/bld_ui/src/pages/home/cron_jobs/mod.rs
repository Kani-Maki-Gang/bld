mod edit;
mod filters;
mod table;

use crate::components::card::Card;
use bld_models::dtos::JobFiltersParams;
use filters::CronJobsFilters;
use leptos::*;
use table::CronJobsTable;

pub use edit::CronJobsEdit;

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
    let refresh = create_rw_signal(());

    let params = move || get_params(limit.get(), pipeline.get());

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="flex justify-items-center gap-x-4 items-center">
                    <div class="grow flex flex-col">
                        <div class="text-2xl">
                            "Cron jobs"
                        </div>
                        <div class="text-gray-400 mb-8">
                            "A list of cron jobs for the current pipelines on the server"
                        </div>
                    </div>
                    <CronJobsFilters
                        limit=limit
                        pipeline=pipeline
                        refresh=refresh />
                </div>
                <CronJobsTable
                    params=params
                    refresh=refresh />
            </div>
        </Card>
    }
}
