use crate::{
    components::{
        button::Button,
        input::{Input, Select, SelectItem},
    },
    context::RefreshHistory,
};
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
    let refresh = use_context::<RefreshHistory>();

    view! {
        <div class="flex items-center gap-x-4">
            <div class="min-w-[400px]">
                <Input placeholder="Search".to_string() value=pipeline />
            </div>
            <div class="min-w-[70px]">
                <Input
                    input_type="number".to_string()
                    placeholder="Limit".to_string()
                    value=limit />
            </div>
            <div class="min-w-[100px]">
                <Select items=states value=state />
            </div>
            <div class="w-32">
                <Button on:click=move |_| if let Some(RefreshHistory(refresh)) = refresh {
                    refresh.set(());
                }>
                    "Apply"
                </Button>
            </div>
        </div>
    }
}
