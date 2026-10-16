mod api;
mod app;
mod browser;
mod components;
mod context;
mod error;
mod format;
mod pages;

use app::App;
use leptos::mount_to_body;

fn main() {
    mount_to_body(App);
}
