use leptos::*;
use stylers::style;
use crate::components::buttons::*;
use crate::components::card::*;

const APP_CLASS: &str = style!{ "App",
    .app {
        display: flex;
        justify-self: center;
    }
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        class = APP_CLASS,
        <div class="app">
            <Card>
                <CardHeader>
                    "Example header"
                </CardHeader>
                <CardBody>
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
                </CardBody>
                <CardFooter>
                    <PrimaryBtn>"Ok"</PrimaryBtn>
                    <button class="bg-secondary">"Ok"</button>
                    <button class="bg-dark">"Ok"</button>
                    <button class="bg-success">"Ok"</button>
                    <button class="bg-danger">"Ok"</button>
                    <button class="bg-warning">"Ok"</button>
                    <AccentBtn>"Ok"</AccentBtn>
                    <AccentLightBtn>"Cancel"</AccentLightBtn>
                </CardFooter>
            </Card>
        </div>
    }
}

