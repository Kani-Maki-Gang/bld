mod details;
mod schedule;

use super::helpers::hash_map_rw_signals;
use crate::{
    components::{button::Button, card::Card},
    error::Error,
    pages::home::RunPipelineVariables,
};
use bld_models::dtos::CronJobResponse;
use bld_runner::VersionedPipeline;
use details::CronJobsEditDetails;
use leptos::{html::Dialog, *};
use schedule::CronJobsEditSchedule;
use std::collections::HashMap;

pub type SaveCronJob = (
    String,
    HashMap<String, RwSignal<String>>,
    HashMap<String, RwSignal<String>>,
);

#[component]
pub fn CronJobsEditErrorDialog(
    #[prop(into)] dialog: NodeRef<Dialog>,
    #[prop(into)] error: Signal<String>,
) -> impl IntoView {
    view! {
        <Card class="flex flex-col gap-4 px-8 py-12 h-[600px] w-[500px]">
            <div class="grow">
                <Error error=move || error.get()/>
            </div>
            <Button on:click=move |_| {
                let _ = dialog.get().map(|x| x.close());
            }>"Close"</Button>
        </Card>
    }
}

#[component]
pub fn CronJobsEdit(
    #[prop(into)] cron: Signal<Option<CronJobResponse>>,
    #[prop(into)] pipeline: Signal<Option<VersionedPipeline>>,
    #[prop(into)] save: WriteSignal<Option<SaveCronJob>>,
    #[prop(into, optional)] delete: Option<WriteSignal<bool>>,
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
            fallback=|| view! { <div class="text-2xl">"Loading..."</div> }
        >
            <div class="flex flex-col gap-4">
                <CronJobsEditDetails
                    job=move || cron.get().unwrap()
                    save=move || save.set(Some(save_data()))
                    delete=delete
                />
                <CronJobsEditSchedule schedule=schedule/>
                <Show when=move || !variables.get().is_empty() fallback=|| view! {}>
                    <RunPipelineVariables
                        title="Variables"
                        subtitle="The variables provided in the cron job run"
                        items=variables
                    />
                </Show>
                <Show when=move || !environment.get().is_empty() fallback=|| view! {}>
                    <RunPipelineVariables
                        title="Environment"
                        subtitle="The environment variables provided in the cron job run"
                        items=environment
                    />
                </Show>
            </div>
        </Show>
    }
}
