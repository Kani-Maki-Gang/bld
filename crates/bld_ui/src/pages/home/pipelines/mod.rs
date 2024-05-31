mod info;
mod run;
mod table;
mod v2;

use crate::components::{button::Button, card::Card};
use leptos::*;
use table::PipelinesTable;

pub use info::PipelineInfo;
pub use run::{variables::RunPipelineVariables, RunPipeline};

#[component]
pub fn Pipelines() -> impl IntoView {
    let refresh = create_rw_signal(());

    provide_context(refresh);

    view! {
        <div class="flex flex-col gap-8 h-full">
            <Card>
                <div class="flex flex-col px-8 py-12">
                    <div class="flex justify-items-center gap-x-4 items-center">
                        <div class="grow flex flex-col">
                            <div class="text-2xl">
                                "Pipelines"
                            </div>
                            <div class="text-gray-400 mb-8">
                                "The list of all available pipelines"
                            </div>
                        </div>
                        <div class="w-40 flex items-end">
                            <Button on:click=move |_| refresh.set(())>
                                "Refresh"
                            </Button>
                        </div>
                    </div>
                    <PipelinesTable />
                </div>
            </Card>
        </div>
    }
}
