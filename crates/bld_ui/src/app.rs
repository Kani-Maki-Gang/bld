use crate::{
    context::{AppDialog, AppDialogContent},
    pages::{
        home::{
            CronJobInsert, CronJobUpdate, CronJobs, Dashboard, History, Home, Monit, PipelineInfo,
            Pipelines, RunPipeline,
        },
        login::Login,
        not_found::NotFound,
        validate::Validate,
    },
};
use leptos::{html::Dialog, *};
use leptos_router::{Route, Router, Routes};

#[component]
pub fn App() -> impl IntoView {
    let app_dialog = create_node_ref::<Dialog>();
    let app_dialog_content: RwSignal<Option<View>> = create_rw_signal(None);

    provide_context(AppDialog(app_dialog));
    provide_context(AppDialogContent(app_dialog_content));

    view! {
        <dialog _ref=app_dialog class="w-full h-full bg-transparent">
            <div class="h-full grid place-items-center">{move || app_dialog_content.get()}</div>
        </dialog>
        <Router>
            <div class="h-screen bg-slate-900">
                <div class="h-screen flex bg-grid">
                    <Routes>
                        <Route path="/" view=Home>
                            <Route path="/" view=Dashboard/>
                            <Route path="/dashboard" view=Dashboard/>
                            <Route path="/history" view=History/>
                            <Route path="/pipelines" view=Pipelines/>
                            <Route path="/pipelines/info" view=PipelineInfo/>
                            <Route path="/pipelines/run" view=RunPipeline/>
                            <Route path="/cron" view=CronJobs/>
                            <Route path="/cron/insert" view=CronJobInsert/>
                            <Route path="/cron/update" view=CronJobUpdate/>
                            <Route path="/monit" view=Monit/>
                        </Route>
                        <Route path="/login" view=Login/>
                        <Route path="/validate" view=Validate/>
                        <Route path="/*any" view=NotFound/>
                    </Routes>
                </div>
            </div>
        </Router>
    }
}
