use crate::{
    components::{
        badge::Chip,
        button::{Button, ButtonSize, ButtonVariant},
        card::{Card, CardHeader},
        input::Input,
        link::{BackLink, LinkButton},
        page::PageHeader,
    },
    format,
    pages::home::pipelines::variables::{RunPipelineVariables, field_values, file_fields},
};
use bld_models::dtos::CronJobResponse;
use bld_runner::VersionedFile;
use leptos::*;
use std::collections::HashMap;

pub type SaveCronJob = (String, HashMap<String, String>, HashMap<String, String>);

#[component]
pub fn CronJobsEdit(
    #[prop()] cron: CronJobResponse,
    #[prop()] pipeline: VersionedFile,
    #[prop(into)] title: String,
    #[prop(into)] on_save: Callback<SaveCronJob>,
    #[prop(into, optional)] on_delete: Option<Callback<()>>,
) -> impl IntoView {
    let schedule = create_rw_signal(cron.schedule.clone());
    let (inputs, env) = file_fields(pipeline, cron.inputs.clone(), cron.env.clone());
    let has_inputs = !inputs.is_empty();
    let has_env = !env.is_empty();
    let (inputs_save, env_save) = (inputs.clone(), env.clone());
    let preview = move || {
        let expr = schedule.get();
        let next = format::cron_next(&expr)
            .map(|d| format!("next run {}", format::until(d)))
            .unwrap_or_else(|| "invalid expression".to_string());
        format!("{} · {next}", format::cron_cadence(&expr))
    };
    let back = format!("/files/info?name={}", cron.pipeline);

    view! {
        <PageHeader
            title=title
            subtitle=cron.pipeline.clone()
            back=view! { <BackLink href="/cron">"Cron jobs"</BackLink> }.into_view()
        >
            <LinkButton href=back variant=ButtonVariant::Ghost size=ButtonSize::Lg>
                "Open pipeline"
            </LinkButton>
            {on_delete
                .map(|on_delete| {
                    view! {
                        <Button
                            variant=ButtonVariant::DangerGhost
                            size=ButtonSize::Lg
                            on:click=move |_| on_delete.call(())
                        >
                            "Delete"
                        </Button>
                    }
                })}
            <Button
                size=ButtonSize::Lg
                on:click=move |_| {
                    on_save
                        .call((
                            schedule.get_untracked(),
                            field_values(&inputs_save),
                            field_values(&env_save),
                        ))
                }
            >
                "Save"
            </Button>
        </PageHeader>
        <div class="flex items-center gap-2 mt-2.5 flex-wrap">
            {cron.is_default.then(|| view! { <Chip>"default"</Chip> })}
            {(!cron.date_created.is_empty())
                .then(|| {
                    view! {
                        <Chip>{format!("created {}", format::short_date(&cron.date_created))}</Chip>
                    }
                })}
            {cron
                .date_updated
                .clone()
                .map(|d| view! { <Chip>{format!("updated {}", format::short_date(&d))}</Chip> })}
        </div>
        <div class="mt-5 grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-3.5 items-start">
            <div class="flex flex-col gap-3.5 min-w-0">
                {has_inputs
                    .then(|| {
                        view! {
                            <Card class="overflow-hidden">
                                <RunPipelineVariables
                                    title="Inputs"
                                    subtitle="The inputs that every scheduled run gets."
                                    fields=inputs
                                />
                            </Card>
                        }
                    })}
                {has_env
                    .then(|| {
                        view! {
                            <Card class="overflow-hidden">
                                <RunPipelineVariables
                                    title="Environment"
                                    subtitle="The environment variables that every scheduled run gets."
                                    fields=env
                                />
                            </Card>
                        }
                    })}
            </div>
            <Card class="overflow-hidden">
                <CardHeader
                    title="Schedule"
                    subtitle="Cron expression with seconds: sec min hour day month weekday."
                />
                <div class="px-5 py-4 flex flex-col gap-2">
                    <Input value=schedule mono=true placeholder="0 */10 * * * *" />
                    <div class="text-[11.5px] text-ink-muted">{preview}</div>
                </div>
            </Card>
        </div>
    }
}
