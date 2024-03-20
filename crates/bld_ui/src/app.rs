use leptos::*;
use leptos_router::{Router, Routes, Route};
use crate::pages::{
    home::{CronJobs, Dashboard, History, Home, Pipelines},
    login::Login, not_found::NotFound
};


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
                            <Route path="/cron" view=CronJobs />
                        </Route>
                        <Route path="/login" view=Login />
                        <Route path="/*any" view=NotFound />
                    </Routes>
                </div>
            </div>
        </Router>
    }
}
