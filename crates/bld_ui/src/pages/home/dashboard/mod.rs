mod kpis;
mod most_runs;

use leptos::*;
use kpis::DashboardKpis;
use most_runs::DashboardMostRunsPerUser;

#[component]
pub fn dashboard() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-12">
            <div class="grid grid-cols-4 justify-items-stretch gap-12">
                <DashboardKpis />
            </div>
            <div class="grid grid-cols-5 justify-items-stretch gap-12">
                <div class="col-span-3"></div>
                <div class="col-span-2">
                    <DashboardMostRunsPerUser />
                </div>
            </div>
        </div>
    }
}
