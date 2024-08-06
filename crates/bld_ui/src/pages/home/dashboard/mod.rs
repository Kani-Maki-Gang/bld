mod kpis;
mod most_runs;
mod pipelines;
mod runs_per_month;

use kpis::DashboardKpis;
use leptos::*;
use most_runs::DashboardMostRunsPerUser;
use pipelines::DashboardPipelines;
use runs_per_month::DashboardRunsPerMonth;

#[component]
pub fn dashboard() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            <div class="grid grid-cols-4 justify-items-stretch gap-4">
                <DashboardKpis/>
            </div>
            <div class="grid grid-cols-5 justify-items-stretch gap-4">
                <div class="col-span-3">
                    <DashboardRunsPerMonth/>
                </div>
                <div class="col-span-2">
                    <DashboardMostRunsPerUser/>
                </div>
            </div>
            <div>
                <DashboardPipelines/>
            </div>
        </div>
    }
}
