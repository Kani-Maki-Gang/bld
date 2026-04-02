mod actions;
mod info;
mod run;
mod table;

use crate::{
    components::{button::IconButton, colors::Colors, input::Input},
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
        <div class="flex flex-col min-h-full">
            <div class="px-6 py-5 border-b border-zinc-800 flex items-center gap-4">
                <div class="grow">
                    <div class="text-lg font-semibold text-white">"Pipelines"</div>
                    <div class="text-xs text-zinc-500 mt-0.5">
                        "All available pipelines on the server"
                    </div>
                </div>
            </div>
            <div class="px-6 py-3 border-b border-zinc-800/60">
                <div class="grid grid-cols-3">
                    <div class="col-span-2">
                        <Input placeholder="Search..." value=filter />
                    </div>
                    <div class="flex justify-end">
                        <IconButton
                            ghost=true
                            color=Colors::Violet
                            icon="iconoir-refresh-double"
                            on:click=move |_| refresh.set()
                        />
                    </div>
                </div>
            </div>
            <div class="px-6 py-5">
                <PipelinesTable filter=move || filter.get() />
            </div>
        </div>
    }
}
