use crate::{
    context::AppDialog,
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
use leptos_router::{Redirect, Route, Router, Routes};

#[component]
pub fn App() -> impl IntoView {
    let node = create_node_ref::<Dialog>();
    let content: RwSignal<Option<View>> = create_rw_signal(None);

    provide_context(AppDialog {
        node,
        content,
        owner: None,
    });

    view! {
        <dialog _ref=node class="bg-transparent p-0 m-auto max-w-none max-h-none overflow-visible">
            {move || content.get()}
        </dialog>
        <Router>
            <Routes>
                <Route path="/" view=Home>
                    <Route path="/" view=|| view! { <Redirect path="/dashboard" /> } />
                    <Route path="/dashboard" view=Dashboard />
                    <Route path="/history" view=History />
                    <Route path="/files" view=Pipelines />
                    <Route path="/files/info" view=PipelineInfo />
                    <Route path="/files/run" view=RunPipeline />
                    <Route path="/cron" view=CronJobs />
                    <Route path="/cron/insert" view=CronJobInsert />
                    <Route path="/cron/update" view=CronJobUpdate />
                    <Route path="/monit" view=Monit />
                </Route>
                <Route path="/login" view=Login />
                <Route path="/validate" view=Validate />
                <Route path="/*any" view=NotFound />
            </Routes>
        </Router>
    }
}
