mod actions;
mod info;
mod run;
mod table;

use crate::{
    components::{button::IconButton, card::Card, input::Input},
    context::RefreshPipelines,
};
use leptos::*;
use table::PipelinesTable;

pub use info::PipelineInfo;
pub use run::{RunPipeline, variables::RunPipelineVariables};

#[component]
pub fn Pipelines() -> impl IntoView {
    let refresh = RefreshPipelines(create_rw_signal(()));
    let filter = create_rw_signal(String::new());

    provide_context(refresh);

    view! {
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12">
                <div class="grid grid-cols-4 pr-2">
                    <div class="grow flex flex-col">
                        <div class="text-2xl">"Pipelines"</div>
                        <div class="text-gray-400 mb-8">"The list of all available pipelines"</div>
                    </div>
                    <div class="col-span-2">
                        <Input placeholder="Search..." value=filter />
                    </div>
                    <div class="flex justify-end">
                        <IconButton icon="iconoir-refresh-double" on:click=move |_| refresh.set() />
                    </div>
                </div>
                <PipelinesTable filter=move || filter.get() />
            </div>
        </Card>
    }
}
