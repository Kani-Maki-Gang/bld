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
        <div class="flex flex-col min-h-full">
            <div class="px-6 py-5 border-b border-zinc-800">
                <div class="text-lg font-semibold text-white">"Dashboard"</div>
                <div class="text-xs text-zinc-500 mt-0.5">"Server overview and pipeline analytics"</div>
            </div>
            <div class="px-6 py-5 flex flex-col gap-5">
                <div class="grid grid-cols-4 gap-4">
                    <DashboardKpis/>
                </div>
                <div class="grid grid-cols-5 gap-4">
                    <div class="col-span-3">
                        <DashboardRunsPerMonth/>
                    </div>
                    <div class="col-span-2">
                        <DashboardMostRunsPerUser/>
                    </div>
                </div>
                <DashboardPipelines/>
            </div>
        </div>
    }
}
