use super::new::CronJobsNewButton;
use crate::{
    components::{button::Button, input::Input},
    context::RefreshCronJobs,
};
use leptos::{leptos_dom::logging, *};

#[component]
pub fn CronJobsFilters(
    #[prop(into)] pipeline: RwSignal<String>,
    #[prop(into)] limit: RwSignal<String>,
) -> impl IntoView {
    let refresh = use_context::<RefreshCronJobs>();
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
                    if let Some(RefreshCronJobs(refresh)) = refresh {
                        refresh.set(());
                    } else {
                        logging::console_error("Refresh cron jobs signal not found in context");
                    }
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
