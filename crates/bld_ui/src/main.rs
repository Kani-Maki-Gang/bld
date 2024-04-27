mod app;
mod components;
mod pages;

use app::App;
use leptos::mount_to_body;

fn main() {
    mount_to_body(App);
}