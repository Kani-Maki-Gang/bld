use leptos::*;
use leptos_router::{Router, Routes, Route};

use crate::pages::{Login, Home};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="h-screen bg-slate-900">
                <div class="h-screen flex bg-grid">
                    <Routes>
                        <Route path="/home" view=Home />
                        <Route path="/login" view=Login />
                        <Route path="/*any" view=Home />
                    </Routes>
                </div>
            </div>
        </Router>
    }
}
