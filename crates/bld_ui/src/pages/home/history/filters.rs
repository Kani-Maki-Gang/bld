use crate::components::input::{Input, Select, SelectItem};
use leptos::*;

#[component]
pub fn HistoryFilters() -> impl IntoView {
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
        <div class="flex flex-row-reverse gap-x-4">
            <div class="min-w-[300px]">
                <Input placeholder="Search".to_string() />
            </div>
            <Select items=states />
            <div class="min-w-[50px]">
                <Input
                    input_type="number".to_string()
                    placeholder="Limit".to_string() />
            </div>
        </div>
    }
}
