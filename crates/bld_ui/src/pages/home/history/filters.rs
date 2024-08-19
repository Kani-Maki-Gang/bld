use crate::components::input::{Input, Select, SelectItem};
use leptos::*;

#[component]
pub fn HistoryFilters(
    #[prop(into)] state: RwSignal<Option<String>>,
    #[prop(into)] limit: RwSignal<String>,
    #[prop(into)] pipeline: RwSignal<String>,
) -> impl IntoView {
    let (states, _set_states) = create_signal(vec![
        SelectItem {
            value: "all".to_string(),
            label: "All".to_string(),
        },
        SelectItem {
            value: "initial".to_string(),
            label: "Initial".to_string(),
        },
        SelectItem {
            value: "queued".to_string(),
            label: "Queued".to_string(),
        },
        SelectItem {
            value: "running".to_string(),
            label: "Running".to_string(),
        },
        SelectItem {
            value: "finished".to_string(),
            label: "Finished".to_string(),
        },
        SelectItem {
            value: "faulted".to_string(),
            label: "Faulted".to_string(),
        },
    ]);

    view! {
        <div class="grid grid-cols-3">
            <div class="col-span-2">
                <Input placeholder="Search..." value=pipeline/>
            </div>
            <div class="flex justify-end gap-4">
                <div class="min-w-[100px]">
                    <Input input_type="number" placeholder="Limit" value=limit/>
                </div>
                <div class="min-w-[100px]">
                    <Select items=states value=state/>
                </div>
            </div>
        </div>
    }
}
