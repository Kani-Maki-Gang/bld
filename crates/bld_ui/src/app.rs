use leptos::*;
use leptos_router::{Router, Routes, Route};

use crate::pages::{Login, Home};
use stylers::style;

const APP_CLASS: &str = style! { "App",
    .app {
        height: 100vh;
        display: flex;
        justify-self: center;
    }
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        class = APP_CLASS,
        <Router>
            <div class="app">
                <Routes>
                    <Route path="/home" view=Home />
                    <Route path="/login" view=Login />
                    <Route path="/*any" view=Login />
                </Routes>
            </div>
        </Router>
    }
}
