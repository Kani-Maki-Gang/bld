mod details;
mod schedule;

use super::helpers::hash_map_rw_signals;
use crate::pages::home::RunPipelineVariables;
use bld_models::dtos::CronJobResponse;
use bld_runner::VersionedPipeline;
use details::CronJobsEditDetails;
use leptos::*;
use schedule::CronJobsEditSchedule;
use std::collections::HashMap;

pub type SaveCronJob = (
    String,
    HashMap<String, RwSignal<String>>,
    HashMap<String, RwSignal<String>>,
);

#[component]
pub fn CronJobsEdit(
    #[prop(into)] cron: Signal<Option<CronJobResponse>>,
    #[prop(into)] pipeline: Signal<Option<VersionedPipeline>>,
    #[prop(into)] save: WriteSignal<Option<SaveCronJob>>,
) -> impl IntoView {
    let schedule = create_rw_signal(String::new());
    let variables = create_rw_signal(HashMap::new());
    let environment = create_rw_signal(HashMap::new());
    let save_data = move || (schedule.get(), variables.get(), environment.get());

    create_effect(move |_| {
        let (Some(cron), Some(pipeline)) = (cron.get(), pipeline.get()) else {
            return;
        };
        schedule.set(cron.schedule);
        let (vars, env) = pipeline.variables_and_environment();
        variables.set(hash_map_rw_signals(vars, cron.variables));
        environment.set(hash_map_rw_signals(env, cron.environment));
    });

    view! {
        <Show
            when=move || cron.get().is_some()
            fallback=|| view! {
                <div class="text-2xl">
                    "Loading..."
                </div>
            }>
            <div class="flex flex-col gap-4">
                <CronJobsEditDetails
                    job=move || cron.get().unwrap()
                    save=move || save.set(Some(save_data())) />
                <CronJobsEditSchedule schedule=schedule />
                <Show
                    when=move || !variables.get().is_empty()
                    fallback=|| view! {}>
                    <RunPipelineVariables
                        title="Variables"
                        subtitle="The variables provided in the cron job run"
                        items=variables />
                </Show>
                <Show
                    when=move || !environment.get().is_empty()
                    fallback=|| view! {}>
                    <RunPipelineVariables
                        title="Environment"
                        subtitle="The environment variables provided in the cron job run"
                        items=environment />
                </Show>
            </div>
        </Show>
    }
}
