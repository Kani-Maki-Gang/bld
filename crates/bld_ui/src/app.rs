use crate::pages::{
    home::{
        CronJobs, CronJobsEdit, Dashboard, History, Home, Monit, PipelineInfo, Pipelines,
        RunPipeline,
    },
    login::Login,
    not_found::NotFound,
};
use leptos::*;
use leptos_router::{Route, Router, Routes};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="h-screen bg-slate-900">
                <div class="h-screen flex bg-grid">
                    <Routes>
                        <Route path="/" view=Home>
                            <Route path="/" view=Dashboard />
                            <Route path="/dashboard" view=Dashboard />
                            <Route path="/history" view=History />
                            <Route path="/pipelines" view=Pipelines />
                            <Route path="/pipelines/info" view=PipelineInfo />
                            <Route path="/pipelines/run" view=RunPipeline />
                            <Route path="/cron" view=CronJobs />
                            <Route path="/cron/edit" view=CronJobsEdit />
                            <Route path="/monit" view=Monit />
                        </Route>
                        <Route path="/login" view=Login />
                        <Route path="/*any" view=NotFound />
                    </Routes>
                </div>
            </div>
        </Router>
    }
}
