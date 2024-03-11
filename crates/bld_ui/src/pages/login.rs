use leptos::*;
use stylers::style;

#[component]
pub fn Login() -> impl IntoView {
    let style = style! {
        "Login",
        .login {
            background: url("grid.svg");
            width: 100%;
            display: flex;
            justify-content: center;
            align-items: center;
        }
        .login-card {
            background-color: var(--dark-color);
            border-radius: 10px;
            min-width: 800px;
            padding: 100px;
            display: flex;
        }
        .logo {
            max-width: 400px;
            max-height: 400px;
            display: flex;
            justify-content: center;
            align-items: center;
        }
        .content {
            border-radius: 10px;
            background-color: var(--primary-color);
            display: flex;
            flex-direction: column;
            align-self: center;
            width: 300px;
            height: 400px;
            margin-left: 100px;
            padding: 24px;
        }
        .title {
            font-size: 32px;
        }
        .subtitle {
            color: var(--text-muted-color);
            font-size: 18px;
            margin-top: 4px;
        }
        .login-btn {
            justify-self: flex-end;
        }
    };

    view! {
        class = style,
        <div class="login">
            <div class="login-card">
                <img class="logo" src="logo.png" />
                <div class="content">
                    <div class="title">
                        "Simple and blazingly fast CI/CD"
                    </div>
                    <div class="subtitle">
                        "Use the below button to redirect to your OIDC provider"
                    </div>
                    <button class="login-btn bg-accent" on:click=move || {

                    }>
                        "Login"
                    </button>
                </div>
            </div>
        </div>
    }
}
