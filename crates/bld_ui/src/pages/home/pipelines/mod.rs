mod actions;
mod cards;
mod info;
mod run;

use crate::{
    components::{input::SearchInput, page::PageHeader},
    context::RefreshPipelines,
};
use cards::PipelineCards;
use leptos::*;

pub use info::PipelineInfo;
pub use run::{RunPipeline, variables};

#[component]
pub fn Pipelines() -> impl IntoView {
    let refresh = RefreshPipelines(create_rw_signal(()));
    let filter = create_rw_signal(String::new());

    provide_context(refresh);

    view! {
        <PageHeader title="Files" subtitle="All pipeline and action files on the server">
            <SearchInput placeholder="Search…" value=filter class="min-w-[200px]" />
        </PageHeader>
        <PipelineCards filter=filter />
    }
}
