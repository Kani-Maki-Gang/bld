use leptos::*;
use stylers::style;

#[component]
pub fn CardHeader(children: Children) -> impl IntoView {
    let style = style! {
        "CardHeader",
        .card-header {
            background-color: var(--dark-color);
            padding: 12px;
        }
    };

    view! {
        class = style,
        <div class="card-header">{children()}</div>
    }
}

#[component]
pub fn CardBody(children: Children) -> impl IntoView {
    let style = style! {
        "CardBody",
        .card-body {
            background-color: var(--primary-color);
            padding: 12px;
        }
    };

    view! {
        class = style,
        <div class="card-body">{children()}</div>
    }
}

#[component]
pub fn CardFooter(children: Children) -> impl IntoView {
    let style = style! {
        "CardFooter",
        .card-footer {
            background-color: var(--primary-color);
            padding: 12px;
        }
    };

    view! {
        class = style,
        <div class="card-footer">{children()}</div>
    }
}


#[component]
pub fn Card(children: Children) -> impl IntoView {
    let style = style! {
        "Card",
        .card {
            background-color: var(--secondary-color);
            border-radius: 5px;
        }
    };

    view! {
        class = style,
        <div class="card">{children()}</div>
    }
}
