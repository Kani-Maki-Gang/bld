use crate::components::{
    button::Button,
    input::{Input, Select, SelectItem},
};
use leptos::*;

#[component]
pub fn HistoryFilters(
    #[prop()] state: RwSignal<Option<String>>,
    #[prop()] limit: RwSignal<Option<String>>,
    #[prop()] pipeline: RwSignal<Option<String>>,
    #[prop()] refresh: RwSignal<()>,
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
        <div class="flex items-center gap-x-4">
            <div class="min-w-[50px]">
                <Input
                    input_type="number".to_string()
                    placeholder="Limit".to_string()
                    value=limit />
            </div>
            <Select items=states value=state />
            <div class="min-w-[300px]">
                <Input placeholder="Search".to_string() value=pipeline />
            </div>
            <div class="w-32">
                <Button on:click=move |_| refresh.set(())>
                    "Apply"
                </Button>
            </div>
        </div>
    }
}
