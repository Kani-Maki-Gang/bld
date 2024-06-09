mod actions;
mod info;
mod run;
mod table;
mod v2;

use crate::{
    components::{button::IconButton, card::Card},
    context::RefreshPipelines,
};
use leptos::*;
use table::PipelinesTable;

pub use info::PipelineInfo;
pub use run::{variables::RunPipelineVariables, RunPipeline};

#[component]
pub fn Pipelines() -> impl IntoView {
    let refresh = RefreshPipelines(create_rw_signal(()));

    provide_context(refresh);

    view! {
        <div class="h-full flex flex-col items-center gap-8">
            <div class="container">
                <Card>
                    <div class="flex flex-col px-8 py-12">
                        <div class="flex items-start gap-x-4 pr-2">
                            <div class="grow flex flex-col">
                                <div class="text-2xl">
                                    "Pipelines"
                                </div>
                                <div class="text-gray-400 mb-8">
                                    "The list of all available pipelines"
                                </div>
                            </div>
                            <IconButton icon="iconoir-refresh-double" on:click=move |_| refresh.set()/>
                        </div>
                        <PipelinesTable />
                    </div>
                </Card>
            </div>
        </div>
    }
}
