mod delete;
mod edit;
mod helpers;
mod insert;
mod new;
mod table;
mod update;

use crate::{
    components::{input::SearchInput, page::PageHeader},
    context::RefreshCronJobs,
};
use leptos::*;
use leptos_router::use_query_map;
use new::CronJobsNewButton;
use table::CronJobsTable;

pub use insert::CronJobInsert;
pub use update::CronJobUpdate;

#[component]
pub fn CronJobs() -> impl IntoView {
    let params = use_query_map();
    let initial = params.with_untracked(|p| p.get("pipeline").cloned().unwrap_or_default());
    let search = create_rw_signal(initial);

    provide_context(RefreshCronJobs(create_rw_signal(())));

    view! {
        <PageHeader title="Cron jobs" subtitle="Scheduled pipeline runs on the server">
            <SearchInput placeholder="Filter by pipeline…" value=search class="min-w-[220px]" />
            <CronJobsNewButton />
        </PageHeader>
        <CronJobsTable search=search />
    }
}
