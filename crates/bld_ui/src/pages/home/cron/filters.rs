use crate::components::{button::Button, input::Input};
use super::new::CronJobsNewButton;
use leptos::*;

#[component]
pub fn CronJobsFilters(
    #[prop(into)] pipeline: RwSignal<String>,
    #[prop(into)] limit: RwSignal<String>,
) -> impl IntoView {
    let refresh = use_context::<RwSignal<()>>();
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
            <div class="w-32">
                <Button on:click=move |_| {
                    let _ = refresh.map(|x| x.set(()));
                }>
                    "Apply"
                </Button>
            </div>
            <div class="w-32">
                <CronJobsNewButton />
            </div>
        </div>
    }
}
