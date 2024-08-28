use crate::{
    api,
    components::{button::{Button, IconButton}, card::Card, input::Input, list::List},
    context::{AppDialog, AppDialogContent},
    error::Error,
};
use bld_models::dtos::ListResponse;
use leptos::{html::Dialog, leptos_dom::logging, *};
use leptos_router::*;

#[component]
fn CronJobsNewDialog(#[prop(into)] app_dialog: NodeRef<Dialog>) -> impl IntoView {
    let search = create_rw_signal(String::new());

    let data = create_resource(
        move || (),
        |_| async move { api::list().await.map_err(|e| e.to_string()) },
    );

    let filtered_pipelines = move || {
        let search = search.get();
        let data = data
            .get()
            .map(|x| x.unwrap_or_default())
            .unwrap_or_default();
        if search.is_empty() {
            data
        } else {
            data.into_iter()
                .filter(|x: &ListResponse| x.pipeline.contains(&search))
                .collect()
        }
    };

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-w-[600px] min-h-[600px]">
                <div class="text-xl">"Add new cron job"</div>
                <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
                    <div class="grow">
                        <Error error=move || data.get().unwrap().unwrap_err()/>
                    </div>
                </Show>
                <Show when=move || matches!(data.get(), Some(Ok(_))) fallback=|| view! {}>
                    <Input placeholder="Pipeline name" value=search/>
                    <div class="grow">
                        <List>
                            <For
                                each=move || {
                                    filtered_pipelines()
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, x)| (i, x.pipeline.clone(), x))
                                }

                                key=|(i, _, _)| *i
                                let:child
                            >
                                <button
                                    class="w-full py-4 px-8 hover:bg-slate-600 hover:cursor-pointer flex items-center"
                                    on:click=move |_| {
                                        let _ = app_dialog.get().map(|x| x.close());
                                        let nav = use_navigate();
                                        nav(
                                            &format!("/cron/insert?name={}", child.1),
                                            NavigateOptions::default(),
                                        );
                                    }
                                >

                                    {child.2.pipeline}
                                </button>
                            </For>
                        </List>
                    </div>
                </Show>
                <Button on:click=move |_| {
                    let _ = app_dialog.get().map(|x| x.close());
                }>"Close"</Button>
            </div>
        </Card>
    }
}

#[component]
pub fn CronJobsNewButton() -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();

    view! {
        <IconButton icon="iconoir-plus" on:click=move |_| {
            let Some(AppDialogContent(content)) = app_dialog_content else {
                logging::console_error("App dialog content is not set");
                return;
            };
            let Some(AppDialog(dialog)) = app_dialog else {
                logging::console_error("App dialog node ref is not set");
                return;
            };
            let _ = dialog.get().map(|x| x.show_modal());
            content.set(Some(view! { <CronJobsNewDialog app_dialog=dialog/> }));
        } />
    }
}
