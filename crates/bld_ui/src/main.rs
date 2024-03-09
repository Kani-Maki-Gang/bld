mod app;
mod components;

use app::App;
use leptos::mount_to_body;

fn main() {
    mount_to_body(App);
}
